//! 受约束的文件写操作（需求 §10.2：恢复/执行/备份共用同一受约束访问层）。
//!
//! - 只写经路径校验的位置；逐组件校验相对路径（§8.5）
//! - 普通复制只处理文件数据：不复刻 ACL、替代数据流、可执行权限（§8.4.8）
//! - 复制保持原始字节：不转换换行、编码或内容（§4.2）

use crate::error::AppResult;
use crate::scanner::FileEntry;
use crate::scanner;
use crate::windows_paths;
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
/// 逐文件校验 sha256，任一不符即失败。
pub fn copy_manifest(src_root: &Path, manifest: &[FileEntry], dst: &Path) -> AppResult<u64> {
    let mut total = 0u64;
    for f in manifest {
        let comps = windows_paths::validate_rel_path(&f.rel_path)?;
        let mut from = src_root.to_path_buf();
        let mut to = dst.to_path_buf();
        for c in &comps {
            from.push(c);
            to.push(c);
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::AppError::from)?;
        }
        std::fs::copy(&from, &to).map_err(crate::error::AppError::from)?;
        let actual = scanner::sha256_file(&to)?;
        if actual != f.sha256 {
            return Err(crate::error::AppError::new(
                crate::error::ErrorCode::IoError,
                format!("暂存文件校验失败：{}", f.rel_path),
            ));
        }
        total += f.size;
    }
    Ok(total)
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
