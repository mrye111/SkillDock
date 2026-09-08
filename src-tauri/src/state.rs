//! 应用状态与事件出口。

use crate::backup::BackupStore;
use crate::error::AppResult;
use crate::executor::EventSink;
use crate::storage::Store;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub data_dir: PathBuf,
    pub store: Store,
    pub backups: BackupStore,
    /// 全局单调递增事件序号（§10.4：前端重连后按 seq 续接）
    pub seq: Arc<AtomicU64>,
    /// 任务取消令牌
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(data_dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(data_dir.join("journals")).map_err(crate::error::AppError::from)?;
        std::fs::create_dir_all(data_dir.join("logs")).map_err(crate::error::AppError::from)?;
        std::fs::create_dir_all(data_dir.join("cache")).map_err(crate::error::AppError::from)?;
        let store = Store::open(data_dir)?;
        let backups = BackupStore::new(data_dir)?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            store,
            backups,
            seq: Arc::new(AtomicU64::new(0)),
            cancels: Mutex::new(HashMap::new()),
        })
    }

    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn register_cancel(&self, task_id: &str) -> Arc<AtomicBool> {
        let token = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(task_id.to_string(), token.clone());
        token
    }

    pub fn request_cancel(&self, task_id: &str) -> bool {
        let map = self.cancels.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = map.get(task_id) {
            t.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn clear_cancel(&self, task_id: &str) {
        self.cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(task_id);
    }
}

/// Tauri 事件出口（命令层注入执行器/扫描器）。
pub struct TauriSink {
    pub app: tauri::AppHandle,
    pub seq: Arc<AtomicU64>,
}

impl EventSink for TauriSink {
    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        if let Err(e) = self.app.emit(event, payload) {
            eprintln!("[skilldock] 事件发送失败 {event}: {e}");
        }
    }
    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }
}
