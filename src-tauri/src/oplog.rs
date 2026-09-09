//! 操作日志：追加式、人类可读的 JSONL，写到「文档\SkillDock\SkillDock-操作日志.jsonl」。
//!
//! 定位：数据库（%LOCALAPPDATA%）是程序状态；本日志是给用户看的永久操作记录，
//! 独立于数据库损坏/重装而存续。写入失败只告警，不影响任务本身。

use serde::Serialize;
use std::path::PathBuf;

/// 文档目录（支持重定向到 OneDrive 等位置；读不到时退回 %USERPROFILE%\Documents）。
pub fn documents_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Documents};
        // SAFETY: 请求当前用户的已知文件夹；raw 由系统分配，随后用 CoTaskMemFree 释放。
        unsafe {
            let mut raw: *mut u16 = std::ptr::null_mut();
            let hr = SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &mut raw);
            if hr != 0 || raw.is_null() {
                return None;
            }
            let mut len = 0;
            while *raw.add(len) != 0 {
                len += 1;
            }
            let s = std::ffi::OsString::from_wide(std::slice::from_raw_parts(raw, len));
            windows_sys::Win32::System::Com::CoTaskMemFree(raw as *const _);
            return Some(PathBuf::from(s));
        }
    }
    #[allow(unreachable_code)]
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Documents"))
}

/// 日志目录：文档\SkillDock（可用 SKILLDOCK_OPS_LOG_DIR 覆盖，测试用）。
pub fn ops_log_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SKILLDOCK_OPS_LOG_DIR") {
        return Some(PathBuf::from(dir));
    }
    documents_dir().map(|d| d.join("SkillDock"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpLogEntry {
    pub time: String,
    pub task_id: String,
    pub kind: String,
    pub trigger: String,
    pub status: String,
    pub library: Option<String>,
    pub duration_ms: Option<u64>,
    pub counts: crate::contract::TaskCounts,
    pub items: Vec<OpLogItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpLogItem {
    pub skill_name: String,
    pub action: String,
    pub status: String,
    pub target_path: String,
    pub error: Option<String>,
}

/// 追加一条任务记录；失败仅告警（日志不得反过来影响任务）。
pub fn append(entry: &OpLogEntry) {
    let Some(dir) = ops_log_dir() else {
        eprintln!("[skilldock] 无法定位文档目录，跳过操作日志");
        return;
    };
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        let line = serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string());
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("SkillDock-操作日志.jsonl"))?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("[skilldock] 操作日志写入失败：{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_jsonl_line() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("SKILLDOCK_OPS_LOG_DIR", tmp.path());
        append(&OpLogEntry {
            time: "2026-09-09T00:00:00Z".into(),
            task_id: "t1".into(),
            kind: "sync".into(),
            trigger: "manual".into(),
            status: "completed".into(),
            library: Some("测试库".into()),
            duration_ms: Some(12),
            counts: crate::contract::TaskCounts {
                success: 1,
                failed: 0,
                skipped: 0,
                conflict_pending: 0,
                cancelled: 0,
                total: 1,
            },
            items: vec![OpLogItem {
                skill_name: "demo".into(),
                action: "create".into(),
                status: "success".into(),
                target_path: "C:\\x\\demo".into(),
                error: None,
            }],
        });
        let content =
            std::fs::read_to_string(tmp.path().join("SkillDock-操作日志.jsonl")).unwrap();
        assert!(content.contains("\"taskId\":\"t1\""));
        assert!(content.contains("demo"));
        std::env::remove_var("SKILLDOCK_OPS_LOG_DIR");
    }
}
