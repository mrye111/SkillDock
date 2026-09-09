//! 受约束的文件写操作（需求 §10.2：恢复/执行/备份共用同一受约束访问层）。
//!
//! - 只写经路径校验的位置；逐组件校验相对路径（§8.5）
//! - 普通复制只处理文件数据：不复刻 ACL、替代数据流、可执行权限（§8.4.8）
//! - 复制保持原始字节：不转换换行、编码或内容（§4.2）

use crate::error::AppResult;
use crate::scanner::FileEntry;
use crate::windows_paths;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 递归复制目录内容（保持字节与结构），返回清单与总字节数。
/// 拒绝重解析点（§8.5）；空目录也按结构复制。
pub fn copy_tree(src: &Path, dst: &Path) -> AppResult<(Vec<FileEntry>, u64)> {
    let mut files = Vec::new();
    let mut total = 0u64;
    copy_tree_inner(src, src, dst, &mut files, &mut total)?;
    Ok((files, total))
}

fn copy_tree_inner(
    root: &Path,
    src: &Path,
    dst: &Path,
    files: &mut Vec<FileEntry>,
    total: &mut u64,
) -> AppResult<()> {
    std::fs::create_dir_all(dst).map_err(crate::error::AppError::from)?;
    let mut entries: Vec<_> = std::fs::read_dir(src)
        .map_err(crate::error::AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::error::AppError::from)?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = std::fs::symlink_metadata(entry.path()).map_err(crate::error::AppError::from)?;
        if windows_paths::is_reparse_point(&meta) {
            return Err(crate::error::AppError::new(
                crate::error::ErrorCode::Unsupported,
                format!("复制源包含符号链接/目录联接：{name}"),
            ));
        }
        let to = dst.join(&name);
        if meta.is_dir() {
            copy_tree_inner(root, &entry.path(), &to, files, total)?;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        // 源端读到的真实文件：路径逐组件校验后才落盘
        let full = entry.path();
        let rel = full
            .strip_prefix(root)
            .unwrap_or(full.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        windows_paths::validate_rel_path(&rel)?;
        std::fs::copy(entry.path(), &to).map_err(crate::error::AppError::from)?;
        *total += meta.len();
        files.push(FileEntry {
            rel_path: rel,
            size: meta.len(),
            sha256: String::new(), // 摘要在调用方统一计算
        });
    }
    Ok(())
}

/// 按清单复制（只复制纳入分发的文件，被忽略规则排除的文件不落盘；§4.3）。
/// 单遍完成：边读源边写暂存边算 sha256——不产生暂存二次读取（§8.4 C 完整校验）。
/// 返回已逐文件核验过的清单条目（调用方直接用于摘要比对）。
pub fn copy_manifest(
    src_root: &Path,
    manifest: &[FileEntry],
    dst: &Path,
) -> AppResult<(Vec<FileEntry>, u64)> {
    use std::io::{Read, Write};
    let mut entries = Vec::with_capacity(manifest.len());
    let mut total = 0u64;
    let mut buf = vec![0u8; 1024 * 1024];
    for f in manifest {
        let comps = windows_paths::validate_rel_path(&f.rel_path)?;
        let mut from_p = src_root.to_path_buf();
        let mut to_p = dst.to_path_buf();
        for c in &comps {
            from_p.push(c);
            to_p.push(c);
        }
        if let Some(parent) = to_p.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::AppError::from)?;
        }
        let mut from = std::fs::File::open(&from_p).map_err(crate::error::AppError::from)?;
        let mut to = std::fs::File::create(&to_p).map_err(crate::error::AppError::from)?;
        let mut h = Sha256::new();
        let mut size = 0u64;
        loop {
            let n = from.read(&mut buf).map_err(crate::error::AppError::from)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            to.write_all(&buf[..n]).map_err(crate::error::AppError::from)?;
            size += n as u64;
        }
        let sha = hex::encode(h.finalize());
        if sha != f.sha256 {
            return Err(crate::error::AppError::new(
                crate::error::ErrorCode::IoError,
                format!("暂存文件校验失败：{}", f.rel_path),
            ));
        }
        total += size;
        entries.push(FileEntry {
            rel_path: f.rel_path.clone(),
            size,
            sha256: sha,
        });
    }
    Ok((entries, total))
}

/// 同卷改名（§8.4：不跨卷移动正式目录；调用方保证同卷）。
pub fn rename_in_place(from: &Path, to: &Path) -> AppResult<()> {
    std::fs::rename(from, to).map_err(crate::error::AppError::from)
}

/// 删除目录树；路径必须位于允许的根之下（防御性检查）。
pub fn remove_tree_under(allowed_root: &Path, path: &Path) -> AppResult<()> {
    windows_paths::ensure_under(allowed_root, path)?;
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(crate::error::AppError::from)?;
    }
    Ok(())
}

/// 目录的父目录（事务目录放在技能集合目录之外的同卷兄弟目录；§8.4.1）。
pub fn sibling_transaction_root(skill_collection_dir: &Path) -> PathBuf {
    skill_collection_dir
        .parent()
        .unwrap_or(skill_collection_dir)
        .join(".skilldock-transactions")
}
