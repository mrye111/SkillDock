//! 统一错误模型（契约 §2）：所有 Tauri 命令与核心服务返回 `Result<T, AppError>`。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    PermissionDenied,
    FileLocked,
    DiskFull,
    PlanStale,
    TargetDrift,
    BackupFailed,
    RecoveryPending,
    InvalidPath,
    NotFound,
    ValidationFailed,
    Unsupported,
    PathOverlap,
    ConflictUnresolved,
    IoError,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<serde_json::Value>,
    pub retryable: bool,
    pub diagnostic_id: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let retryable = matches!(code, ErrorCode::FileLocked | ErrorCode::IoError);
        Self {
            code,
            message: message.into(),
            context: None,
            retryable,
            diagnostic_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn not_found(what: &str, id: &str) -> Self {
        Self::new(ErrorCode::NotFound, format!("找不到{what}：{id}"))
            .with_context(serde_json::json!({ "kind": what, "id": id }))
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }

    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidPath, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

/// Windows 原始错误码 → 契约错误码（§2 要求至少区分权限/占用/空间/路径）。
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        let (code, hint) = match e.raw_os_error() {
            Some(5) => (ErrorCode::PermissionDenied, "权限不足"),
            Some(32) | Some(33) => (ErrorCode::FileLocked, "文件被其他进程占用"),
            Some(112) => (ErrorCode::DiskFull, "磁盘空间不足"),
            Some(206) => (ErrorCode::InvalidPath, "路径过长"),
            _ => match e.kind() {
                std::io::ErrorKind::NotFound => (ErrorCode::NotFound, "路径不存在"),
                std::io::ErrorKind::PermissionDenied => (ErrorCode::PermissionDenied, "权限不足"),
                _ => (ErrorCode::IoError, "文件系统错误"),
            },
        };
        Self::new(code, format!("{hint}：{e}"))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::internal(format!("数据库错误：{e}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(format!("JSON 序列化错误：{e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;
