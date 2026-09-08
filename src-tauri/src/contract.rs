//! 前后端契约的线类型（与 docs/api-contract.md、src/lib/backend-contract.ts 同步）。
//! 所有载荷 camelCase；所有枚举值 snake_case。

use serde::{Deserialize, Serialize};

pub use crate::scanner::{SourceCandidate, ValidationIssue, ValidationStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCellState {
    NoMapping,
    ToAdd,
    SameContentUnmanaged,
    UnmanagedConflict,
    Synced,
    SourceUpdated,
    TargetModified,
    BothModified,
    AlignedExternally,
    TargetDeleted,
    SourceRemoved,
    OwnershipConflict,
    Invalid,
    Unsupported,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanAction {
    Create,
    Update,
    Adopt,
    Overwrite,
    TakeOver,
    Reinstall,
    Remove,
    Restore,
    Skip,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictChoice {
    KeepTarget,
    OverwriteWithSource,
    AdoptExisting,
    TakeOver,
    PauseMapping,
    Reinstall,
    KeepDeleted,
    TransferOwnership,
    RemoveWithBackup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Partial,
    Failed,
    Cancelled,
    RecoveryPending,
}

impl TaskStatus {
    pub fn is_final(self) -> bool {
        !matches!(self, TaskStatus::Queued | TaskStatus::Running)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::RecoveryPending => "recovery_pending",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskItemStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    ConflictPending,
    Cancelled,
    RecoveryPending,
}

impl TaskItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::ConflictPending => "conflict_pending",
            Self::Cancelled => "cancelled",
            Self::RecoveryPending => "recovery_pending",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "success" => Self::Success,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            "conflict_pending" => Self::ConflictPending,
            "cancelled" => Self::Cancelled,
            "recovery_pending" => Self::RecoveryPending,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Scan,
    Sync,
    Remove,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    Manual,
    Retry,
    Restore,
    StartupRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanOperation {
    Sync,
    Remove,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Active,
    Stale,
    Executed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetScope {
    User,
    Project,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Exists,
    WillCreate,
    NoPermission,
    InvalidPath,
    UnsupportedLocation,
}

// ---------------------------------------------------------------------------
// 4.1 技能库
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterLibraryInput {
    pub path: String,
    pub display_name: Option<String>,
    pub mode: Option<String>,
    pub selected_root: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathDiagnostic {
    pub level: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub library_id: String,
    pub display_name: String,
    pub canonical_path: String,
    pub source_root: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub path_valid: bool,
    pub skill_count: u32,
    pub invalid_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterLibraryResult {
    pub library_id: String,
    pub canonical_path: String,
    pub source_root: Option<String>,
    pub needs_root_choice: bool,
    pub candidates: Vec<SourceCandidate>,
    pub diagnostics: Vec<PathDiagnostic>,
    pub recent_libraries: Vec<LibrarySummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCell {
    pub state: MatrixCellState,
    pub mapping_id: Option<String>,
    pub target_dir_name: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillValidation {
    pub status: ValidationStatus,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    pub skill_id: String,
    pub rel_path: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    pub last_content_changed_at: Option<String>,
    pub digest: Option<String>,
    pub validation: SkillValidation,
    pub targets: std::collections::HashMap<String, MatrixCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDetail {
    pub summary: LibrarySummary,
    pub ignore_patterns: Vec<String>,
    pub config_version: i64,
    pub skills: Vec<SkillView>,
}

// ---------------------------------------------------------------------------
// 4.2 目标管理
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPath {
    pub path: String,
    pub source: String,
    pub availability: Availability,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterInfo {
    pub adapter_id: String,
    pub display_name: String,
    pub adapter_version: String,
    pub verified_at: String,
    pub docs_url: String,
    pub scopes: Vec<TargetScope>,
    pub user_template: Option<String>,
    pub project_template: Option<String>,
    pub resolved: AdapterResolved,
    pub detected_hints: Vec<String>,
    pub also_read_by: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterResolved {
    pub user: Option<ResolvedPath>,
    pub project: Option<ResolvedPath>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTargetInput {
    pub adapter_id: String,
    pub scope: TargetScope,
    pub path: Option<String>,
    pub project_root: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTargetResult {
    pub target_id: String,
    pub physical_target_id: String,
    pub resolved_path: String,
    pub resolution_source: String,
    pub availability: Availability,
    pub shared_with_tools: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetView {
    pub target_id: String,
    pub physical_target_id: String,
    pub adapter_id: String,
    pub display_name: String,
    pub scope: TargetScope,
    pub resolved_path: String,
    pub resolution_source: String,
    pub availability: Availability,
    pub shared_with_tools: Vec<String>,
    pub mapped_skill_count: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingView {
    pub mapping_id: String,
    pub skill_id: String,
    pub physical_target_id: String,
    pub target_dir_name: String,
    pub enabled: bool,
    pub paused_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// 4.3 计划与执行
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSyncPlanInput {
    pub library_id: String,
    pub operation: PlanOperation,
    pub mapping_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub kind: String, // add | update | delete
    pub rel_path: String,
    pub bytes: u64,
    pub source_digest: Option<String>,
    pub target_digest: Option<String>,
    pub diff_previewable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub kind: String, // unmanaged_same_name | same_content | target_modified | both_modified | ownership | restore_drift
    pub message: String,
    pub available_choices: Vec<ConflictChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub item_id: String,
    pub mapping_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub rel_path: String,
    pub action: PlanAction,
    pub file_changes: Vec<FileChange>,
    pub excluded_by_ignore: Vec<ExcludedFile>,
    pub needs_backup: bool,
    pub target_path: String,
    pub state_before: MatrixCellState,
    pub conflict: Option<ConflictInfo>,
    pub blocked_reason: Option<String>,
    pub selected: bool,
    pub decision: Option<ConflictChoice>,
    /// 计划生成时的摘要指纹（执行前复核用）
    pub source_digest: Option<String>,
    pub target_digest: Option<String>,
    pub baseline_digest: Option<Option<String>>,
    /// restore 专用：恢复快照 ID 与「恢复为不存在」标记
    #[serde(default)]
    pub restore: Option<RestoreSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedFile {
    pub rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSpec {
    pub snapshot_id: String,
    pub to_absent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanGroup {
    pub physical_target_id: String,
    pub target_path: String,
    pub shared_with_tools: Vec<String>,
    pub items: Vec<PlanItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub create_count: u32,
    pub update_count: u32,
    pub remove_count: u32,
    pub restore_count: u32,
    pub skip_count: u32,
    pub conflict_count: u32,
    pub blocked_count: u32,
    pub executable_count: u32,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub plan_id: String,
    pub plan_version: i64,
    pub library_id: String,
    pub operation: PlanOperation,
    pub status: PlanStatus,
    pub created_at: String,
    pub config_version: i64,
    pub groups: Vec<PlanGroup>,
    pub summary: PlanSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCounts {
    pub success: u32,
    pub failed: u32,
    pub skipped: u32,
    pub conflict_pending: u32,
    pub cancelled: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskItemView {
    pub item_id: String,
    pub mapping_id: Option<String>,
    pub skill_name: String,
    pub target_path: String,
    pub action: PlanAction,
    pub status: TaskItemStatus,
    pub error: Option<crate::error::AppError>,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshotView {
    pub task_id: String,
    pub kind: TaskKind,
    pub trigger: TaskTrigger,
    pub status: TaskStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub counts: TaskCounts,
    pub items: Vec<TaskItemView>,
    pub error: Option<crate::error::AppError>,
}

// ---------------------------------------------------------------------------
// 4.4 历史与恢复
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub library_id: Option<String>,
    pub target_id: Option<String>,
    pub status: Option<TaskStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTaskSummary {
    pub task_id: String,
    pub kind: TaskKind,
    pub trigger: TaskTrigger,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub status: TaskStatus,
    pub counts: TaskCounts,
    pub library_name: Option<String>,
    pub target_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryTaskSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRestorePlanInput {
    pub task_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStats {
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub protected_bytes: u64,
    pub retention_days: u32,
    pub soft_cap_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotView {
    pub snapshot_id: String,
    pub task_item_id: Option<String>,
    pub mapping_id: Option<String>,
    pub target_path: String,
    pub existed_before: bool,
    pub digest: Option<String>,
    pub bytes: u64,
    pub pinned: bool,
    pub available: bool,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// 5. 事件
// ---------------------------------------------------------------------------

// 事件名：Tauri 2 只允许字母数字、'-'、'/'、':'、'_'（点号不合法）
pub const EVENT_SCAN_PROGRESS: &str = "scan://progress";
pub const EVENT_SYNC_PROGRESS: &str = "sync://progress";
pub const EVENT_SYNC_COMPLETED: &str = "sync://completed";
pub const EVENT_RECOVERY_REQUIRED: &str = "recovery://required";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    pub task_id: String,
    pub seq: u64,
    pub scanned_dirs: u32,
    pub discovered_skills: u32,
    pub invalid_count: u32,
    pub current_path: Option<String>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressEvent {
    pub task_id: String,
    pub seq: u64,
    pub phase: String,
    pub item_index: u32,
    pub item_count: u32,
    pub item_id: Option<String>,
    pub skill_name: Option<String>,
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCompletedEvent {
    pub task_id: String,
    pub seq: u64,
    pub status: TaskStatus,
    pub counts: TaskCounts,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRequiredEvent {
    pub task_id: Option<String>,
    pub seq: u64,
    pub transaction_id: String,
    pub target_path: String,
    pub skill_name: Option<String>,
    pub reason: String,
    pub recoverable: String, // auto | manual
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tauri 2 事件名校验：只允许字母数字、'-'、'/'、':'、'_'
    /// （点号曾在 v1.0 导致所有事件发送失败，回归测试）
    #[test]
    fn event_names_are_tauri_legal() {
        for name in [
            EVENT_SCAN_PROGRESS,
            EVENT_SYNC_PROGRESS,
            EVENT_SYNC_COMPLETED,
            EVENT_RECOVERY_REQUIRED,
        ] {
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | ':' | '_')),
                "事件名 {name} 含 Tauri 不允许的字符"
            );
        }
    }
}
