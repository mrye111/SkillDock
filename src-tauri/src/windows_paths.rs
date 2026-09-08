//! 受约束文件系统访问的路径层（需求 §5.2、§8.5、AC-25/26）。
//!
//! 职责：Windows 路径规范化与身份判定、相对子项合法性校验、重解析点检测、
//! 根目录重叠判定、不支持位置（UNC/网络映射盘）识别、超长路径预检。
//! 路径展示与实际访问分开处理：本模块产出的是访问用的规范化路径。

use crate::error::{AppError, ErrorCode};
use std::path::{Component, Path, PathBuf};

/// Windows 保留设备名（不区分大小写；含带扩展名形式如 `CON.txt`）。
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 相对路径内禁止出现的字符（Windows 文件系统规则）。
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

/// 规范化路径：解析 `.` / 符号链接到真实位置，去掉 `\\?\` 前缀。
/// 目标不存在时退化为对最长存在前缀的规范化 + 追加剩余部分。
pub fn canonicalize(p: &Path) -> Result<PathBuf, AppError> {
    let canon = match std::fs::canonicalize(p) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => canonicalize_lenient(p)?,
        Err(e) => return Err(AppError::from(e)),
    };
    Ok(strip_verbatim_prefix(&canon))
}

/// 对不存在的路径做尽力规范化：找到最长已存在祖先，规范化后拼回剩余组件。
fn canonicalize_lenient(p: &Path) -> Result<PathBuf, AppError> {
    let mut existing = p.to_path_buf();
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if existing.exists() {
            let mut base = std::fs::canonicalize(&existing).map_err(AppError::from)?;
            for comp in missing.iter().rev() {
                base.push(comp);
            }
            return Ok(base);
        }
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => return Ok(p.to_path_buf()),
        }
    }
}

/// 去掉 `\\?\`（及 `\\?\UNC\`）前缀，便于展示与比较。
pub fn strip_verbatim_prefix(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p.to_path_buf()
    }
}

/// 目录身份：规范化 + 大小写折叠 + 分隔符统一 + 去尾部分隔符（§5.2.6）。
/// 目录联接经 canonicalize 解析到真实位置，因此同一真实目录得到同一身份。
pub fn dir_identity(p: &Path) -> Result<String, AppError> {
    Ok(fold_case(&canonicalize(p)?))
}

/// 大小写折叠的路径字符串（用于同一位置比较，不用于访问）。
pub fn fold_case(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    let s = s.trim_end_matches('\\');
    s.to_lowercase()
}

/// 校验一个相对子路径（如 `code-review/references/x.md`）的每个组件（§8.5）。
pub fn validate_rel_path(rel: &str) -> Result<Vec<String>, AppError> {
    if rel.is_empty() {
        return Err(AppError::invalid_path("相对路径为空"));
    }
    let mut out = Vec::new();
    for raw in rel.split(['/', '\\']) {
        let comp = raw;
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp == ".." {
            return Err(AppError::invalid_path(format!("路径包含越界组件 `..`：{rel}")));
        }
        if comp.contains(':') {
            return Err(AppError::invalid_path(format!("路径包含盘符/流语法：{rel}")));
        }
        if comp.chars().any(|c| ILLEGAL_CHARS.contains(&c) || (c as u32) < 0x20) {
            return Err(AppError::invalid_path(format!("路径包含非法字符：{comp}")));
        }
        if comp.ends_with('.') || comp.ends_with(' ') {
            return Err(AppError::invalid_path(format!(
                "路径组件以点或空格结尾，存在歧义：{comp}"
            )));
        }
        let stem = comp.split('.').next().unwrap_or(comp);
        if RESERVED_NAMES.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
            return Err(AppError::invalid_path(format!("路径组件为保留设备名：{comp}")));
        }
        out.push(comp.to_string());
    }
    if out.is_empty() {
        return Err(AppError::invalid_path(format!("相对路径无效：{rel}")));
    }
    Ok(out)
}

/// 绝对路径是否指向设备命名空间（`\\.\`、`\\?\` 全局根等）；规范化后的路径不应出现。
pub fn is_device_path(p: &Path) -> bool {
    let s = p.to_string_lossy();
    s.starts_with(r"\\.\") || s.starts_with(r"\\?\")
}

/// 是否 UNC 路径（`\\server\share`）。
pub fn is_unc(p: &Path) -> bool {
    matches!(p.components().next(), Some(Component::Prefix(_)))
        && p.to_string_lossy().starts_with(r"\\")
        && !p.to_string_lossy().starts_with(r"\\?\")
}

/// 根路径所在驱动器是否为网络映射盘（DRIVE_REMOTE）。非 Windows 或读取失败返回 false。
#[cfg(windows)]
pub fn is_remote_drive(p: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let root: PathBuf = p
        .components()
        .next()
        .map(|c| PathBuf::from(c.as_os_str()))
        .unwrap_or_else(|| p.to_path_buf());
    let mut root_str = root.to_string_lossy().to_string();
    if !root_str.ends_with('\\') {
        root_str.push('\\');
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(&root_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: 以 NUL 结尾的宽字符缓冲区，调用只读的 GetDriveTypeW。
    unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr()) == 4 }
}

#[cfg(not(windows))]
pub fn is_remote_drive(_p: &Path) -> bool {
    false
}

/// 位置是否不支持（§5.2.7：UNC、网络映射盘；云占位由调用方另行提示）。
pub fn unsupported_location_reason(p: &Path) -> Option<String> {
    if is_device_path(p) {
        return Some("设备命名空间路径不受支持".to_string());
    }
    if is_unc(p) {
        return Some("UNC 网络路径不受支持，请使用本地磁盘目录".to_string());
    }
    if is_remote_drive(p) {
        return Some("网络映射盘不受支持，请使用本地磁盘目录".to_string());
    }
    None
}

/// 元数据是否为重解析点（符号链接、目录联接等；§8.5 不跟随）。
pub fn is_reparse_point(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// 检查路径是否处于重解析点之下或本身是重解析点（遍历前与提交前各检查一次）。
pub fn contains_reparse_point(p: &Path) -> Result<bool, AppError> {
    let mut cur = PathBuf::new();
    for comp in p.components() {
        cur.push(comp.as_os_str());
        match std::fs::symlink_metadata(&cur) {
            Ok(meta) => {
                if is_reparse_point(&meta) {
                    return Ok(true);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(AppError::from(e)),
        }
    }
    Ok(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    Same,
    /// a 是 b 的祖先
    AContainsB,
    /// b 是 a 的祖先
    BContainsA,
    Disjoint,
}

/// 判断两个根目录（已规范化）是否相同或互相包含（§5.2.5）。
pub fn overlap(a: &Path, b: &Path) -> Overlap {
    let fa = fold_case(a);
    let fb = fold_case(b);
    if fa == fb {
        return Overlap::Same;
    }
    let (mut pa, mut pb) = (fa.clone(), fb.clone());
    pa.push('\\');
    pb.push('\\');
    if pb.starts_with(&pa) {
        return Overlap::AContainsB;
    }
    if pa.starts_with(&pb) {
        return Overlap::BContainsA;
    }
    Overlap::Disjoint
}

/// 查询路径所在卷的可用字节数（§8.5 空间预检）。路径不存在时向上找最近存在祖先。
#[cfg(windows)]
pub fn free_space(p: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    let mut cur = p.to_path_buf();
    loop {
        if cur.exists() {
            let wide: Vec<u16> = std::ffi::OsStr::new(&cur.as_os_str())
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut free: u64 = 0;
            // SAFETY: wide 以 NUL 结尾；free 为有效的 u64 写出指针。
            let ok = unsafe {
                windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                    wide.as_ptr(),
                    &mut free,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            return if ok != 0 { Some(free) } else { None };
        }
        if !cur.pop() {
            return None;
        }
    }
}

#[cfg(not(windows))]
pub fn free_space(_p: &Path) -> Option<u64> {
    None
}

/// 超长路径预检（§8.5）：返回 true 表示超过保守阈值（240 字符），需要缩短路径建议。
pub fn is_long_path(p: &Path) -> bool {
    p.to_string_lossy().chars().count() > 240
}

/// 把校验过的相对组件安全地拼到根目录下，并确认结果仍在根内。
pub fn join_under(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    let comps = validate_rel_path(rel)?;
    let mut out = root.to_path_buf();
    for c in comps {
        out.push(c);
    }
    Ok(out)
}

/// 校验目标完整路径可写区域：最终路径不得越出给定根目录。
pub fn ensure_under(root: &Path, child: &Path) -> Result<(), AppError> {
    match overlap(root, child) {
        Overlap::AContainsB | Overlap::Same => Ok(()),
        _ => Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("路径越界：{} 不在 {} 之内", child.display(), root.display()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_reserved_names() {
        assert!(validate_rel_path("../x").is_err());
        assert!(validate_rel_path("a/../../b").is_err());
        assert!(validate_rel_path("CON").is_err());
        assert!(validate_rel_path("con.txt").is_err());
        assert!(validate_rel_path("a/trailing./b").is_err());
        assert!(validate_rel_path("a/trailing ").is_err());
        assert!(validate_rel_path("a:b").is_err());
        assert!(validate_rel_path("").is_err());
        assert!(validate_rel_path("ok-dir/sub_file.md").unwrap().len() == 2);
        assert!(validate_rel_path("中文 目录/文件 名.md").is_ok());
    }

    #[test]
    fn fold_case_compares_equal() {
        let a = Path::new(r"C:\Users\X\Skills");
        let b = Path::new(r"c:\users\x\skills\");
        assert_eq!(fold_case(a), fold_case(b));
    }

    #[test]
    fn overlap_detects_containment() {
        let root = Path::new(r"C:\a\b");
        let inside = Path::new(r"C:\a\b\c");
        let other = Path::new(r"C:\a\bd");
        assert_eq!(overlap(root, root), Overlap::Same);
        assert_eq!(overlap(root, inside), Overlap::AContainsB);
        assert_eq!(overlap(inside, root), Overlap::BContainsA);
        assert_eq!(overlap(root, other), Overlap::Disjoint);
    }
}
