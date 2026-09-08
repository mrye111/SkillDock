/**
 * SkillDock 前后端接口契约 —— TypeScript 类型（前端专用）
 *
 * 版本：1.0 · 2026-09-08 · 维护方：后端会话
 * 权威文档：docs/api-contract.md（语义以该文档为准，二者同步维护）
 *
 * 约定：
 * - 命令名 snake_case；载荷字段 camelCase；枚举值 snake_case
 * - 所有 ID 为后端生成的字符串；时间为 ISO 8601 UTC 字符串
 * - 所有命令失败时 reject AppError
 */

// ---------------------------------------------------------------------------
// 2. 错误模型
// ---------------------------------------------------------------------------

export type ErrorCode =
  | 'permission_denied'
  | 'file_locked'
  | 'disk_full'
  | 'plan_stale'
  | 'target_drift'
  | 'backup_failed'
  | 'recovery_pending'
  | 'invalid_path'
  | 'not_found'
  | 'validation_failed'
  | 'unsupported'
  | 'path_overlap'
  | 'conflict_unresolved'
  | 'io_error'
  | 'internal';

export interface AppError {
  code: ErrorCode;
  /** 用户可读消息：含受影响技能、目标路径、实际原因与下一步建议 */
  message: string;
  context: Record<string, unknown> | null;
  retryable: boolean;
  diagnosticId: string;
}

// ---------------------------------------------------------------------------
// 3. 核心枚举
// ---------------------------------------------------------------------------

/** 矩阵单元格状态（需求 §8.2 决策表的展示态） */
export type MatrixCellState =
  | 'no_mapping'
  | 'to_add'
  | 'same_content_unmanaged'
  | 'unmanaged_conflict'
  | 'synced'
  | 'source_updated'
  | 'target_modified'
  | 'both_modified'
  | 'aligned_externally'
  | 'target_deleted'
  | 'source_removed'
  | 'ownership_conflict'
  | 'invalid'
  | 'unsupported'
  | 'paused';

export type ValidationStatus = 'valid' | 'invalid' | 'unsupported';

export type PlanAction =
  | 'create'
  | 'update'
  | 'adopt'
  | 'overwrite'
  | 'take_over'
  | 'reinstall'
  | 'remove'
  | 'restore'
  | 'skip'
  | 'blocked';

export type ConflictChoice =
  | 'keep_target'
  | 'overwrite_with_source'
  | 'adopt_existing'
  | 'take_over'
  | 'pause_mapping'
  | 'reinstall'
  | 'keep_deleted'
  | 'transfer_ownership';

export type TaskStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'partial'
  | 'failed'
  | 'cancelled'
  | 'recovery_pending';

export type TaskItemStatus =
  | 'pending'
  | 'running'
  | 'success'
  | 'failed'
  | 'skipped'
  | 'conflict_pending'
  | 'cancelled'
  | 'recovery_pending';

export type TaskKind = 'scan' | 'sync' | 'remove' | 'restore';
export type TaskTrigger = 'manual' | 'retry' | 'restore' | 'startup_recovery';
export type PlanOperation = 'sync' | 'remove' | 'restore';
export type PlanStatus = 'active' | 'stale' | 'executed' | 'cancelled';
export type TargetScope = 'user' | 'project' | 'custom';
export type Availability =
  | 'exists'
  | 'will_create'
  | 'no_permission'
  | 'invalid_path'
  | 'unsupported_location';

export type AdapterId = 'codex' | 'claude_code' | 'cursor' | 'copilot_vscode' | 'custom';

// ---------------------------------------------------------------------------
// 4.1 技能库
// ---------------------------------------------------------------------------

export interface RegisterLibraryInput {
  path: string;
  displayName?: string;
  mode?: 'auto' | 'collection' | 'single';
  selectedRoot?: string;
}

export interface SourceCandidate {
  path: string;
  origin: 'root' | 'skills' | 'agents_skills' | 'claude_skills' | 'custom';
  validSkillCount: number;
  invalidSkillCount: number;
}

export interface PathDiagnostic {
  level: 'info' | 'warning' | 'error';
  code: string;
  message: string;
}

export interface LibrarySummary {
  libraryId: string;
  displayName: string;
  canonicalPath: string;
  sourceRoot: string | null;
  pinned: boolean;
  lastOpenedAt: string | null;
  pathValid: boolean;
  skillCount: number;
  invalidCount: number;
}

export interface RegisterLibraryResult {
  libraryId: string;
  canonicalPath: string;
  sourceRoot: string | null;
  needsRootChoice: boolean;
  candidates: SourceCandidate[];
  diagnostics: PathDiagnostic[];
  recentLibraries: LibrarySummary[];
}

export interface ValidationIssue {
  code: string;
  message: string;
  line: number | null;
  column: number | null;
}

export interface MatrixCell {
  state: MatrixCellState;
  mappingId: string | null;
  targetDirName: string | null;
  detail: string | null;
}

export interface SkillView {
  skillId: string;
  relPath: string;
  name: string | null;
  description: string | null;
  fileCount: number;
  totalBytes: number;
  lastContentChangedAt: string | null;
  digest: string | null;
  validation: {
    status: ValidationStatus;
    issues: ValidationIssue[];
  };
  targets: Record<string, MatrixCell>;
}

export interface LibraryDetail {
  summary: LibrarySummary;
  ignorePatterns: string[];
  configVersion: number;
  skills: SkillView[];
}

// ---------------------------------------------------------------------------
// 4.2 目标管理
// ---------------------------------------------------------------------------

export interface ResolvedPath {
  path: string;
  source: string;
  availability: Availability;
  detail: string | null;
}

export interface AdapterInfo {
  adapterId: AdapterId;
  displayName: string;
  adapterVersion: string;
  verifiedAt: string;
  docsUrl: string;
  scopes: TargetScope[];
  userTemplate: string | null;
  projectTemplate: string | null;
  resolved: {
    user: ResolvedPath | null;
    project: ResolvedPath | null;
  };
  detectedHints: string[];
  alsoReadBy: string[];
  notes: string[];
}

export interface SaveTargetInput {
  adapterId: AdapterId;
  scope: TargetScope;
  path?: string;
  projectRoot?: string;
  displayName?: string;
}

export interface SaveTargetResult {
  targetId: string;
  physicalTargetId: string;
  resolvedPath: string;
  resolutionSource: string;
  availability: Availability;
  sharedWithTools: string[];
  warnings: string[];
}

export interface TargetView {
  targetId: string;
  physicalTargetId: string;
  adapterId: string;
  displayName: string;
  scope: TargetScope;
  resolvedPath: string;
  resolutionSource: string;
  availability: Availability;
  sharedWithTools: string[];
  mappedSkillCount: number;
  enabled: boolean;
}

export interface MappingView {
  mappingId: string;
  skillId: string;
  physicalTargetId: string;
  targetDirName: string;
  enabled: boolean;
  pausedReason: string | null;
}

// ---------------------------------------------------------------------------
// 4.3 同步计划与执行
// ---------------------------------------------------------------------------

export interface CreateSyncPlanInput {
  libraryId: string;
  operation: PlanOperation;
  mappingIds: string[];
}

export interface FileChange {
  kind: 'add' | 'update' | 'delete';
  relPath: string;
  bytes: number;
  sourceDigest: string | null;
  targetDigest: string | null;
  diffPreviewable: boolean;
}

export interface ConflictInfo {
  kind: 'unmanaged_same_name' | 'target_modified' | 'both_modified' | 'ownership' | 'restore_drift';
  message: string;
  availableChoices: ConflictChoice[];
}

export interface PlanItemView {
  itemId: string;
  mappingId: string;
  skillId: string;
  skillName: string;
  relPath: string;
  action: PlanAction;
  fileChanges: FileChange[];
  excludedByIgnore: { relPath: string }[];
  needsBackup: boolean;
  targetPath: string;
  stateBefore: MatrixCellState;
  conflict: ConflictInfo | null;
  blockedReason: string | null;
  selected: boolean;
  decision: ConflictChoice | null;
}

export interface PlanGroupView {
  physicalTargetId: string;
  targetPath: string;
  sharedWithTools: string[];
  items: PlanItemView[];
}

export interface PlanSummary {
  createCount: number;
  updateCount: number;
  removeCount: number;
  restoreCount: number;
  skipCount: number;
  conflictCount: number;
  blockedCount: number;
  executableCount: number;
  totalBytes: number;
}

export interface SyncPlanView {
  planId: string;
  planVersion: number;
  libraryId: string;
  operation: PlanOperation;
  status: PlanStatus;
  createdAt: string;
  configVersion: number;
  groups: PlanGroupView[];
  summary: PlanSummary;
}

export interface TaskCounts {
  success: number;
  failed: number;
  skipped: number;
  conflictPending: number;
  cancelled: number;
  total: number;
}

export interface TaskItemView {
  itemId: string;
  mappingId: string | null;
  skillName: string;
  targetPath: string;
  action: PlanAction;
  status: TaskItemStatus;
  error: AppError | null;
  bytesProcessed: number;
  bytesTotal: number;
  snapshotId: string | null;
}

export interface TaskSnapshot {
  taskId: string;
  kind: TaskKind;
  trigger: TaskTrigger;
  status: TaskStatus;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  counts: TaskCounts;
  items: TaskItemView[];
  error: AppError | null;
}

// ---------------------------------------------------------------------------
// 4.4 历史与恢复
// ---------------------------------------------------------------------------

export interface HistoryQuery {
  cursor?: string;
  limit?: number;
  libraryId?: string;
  targetId?: string;
  status?: TaskStatus;
}

export interface HistoryTaskSummary {
  taskId: string;
  kind: TaskKind;
  trigger: TaskTrigger;
  startedAt: string | null;
  durationMs: number | null;
  status: TaskStatus;
  counts: TaskCounts;
  libraryName: string | null;
  targetPaths: string[];
}

export interface HistoryPage {
  items: HistoryTaskSummary[];
  nextCursor: string | null;
}

export interface CreateRestorePlanInput {
  taskId: string;
  itemIds: string[];
}

export interface BackupStats {
  totalBytes: number;
  reclaimableBytes: number;
  protectedBytes: number;
  retentionDays: number;
  softCapBytes: number;
}

export interface SnapshotView {
  snapshotId: string;
  taskItemId: string | null;
  mappingId: string | null;
  targetPath: string;
  existedBefore: boolean;
  digest: string | null;
  bytes: number;
  pinned: boolean;
  available: boolean;
  createdAt: string;
}

// ---------------------------------------------------------------------------
// 5. 事件
// ---------------------------------------------------------------------------

export const EVENT_SCAN_PROGRESS = 'scan.progress';
export const EVENT_SYNC_PROGRESS = 'sync.progress';
export const EVENT_SYNC_COMPLETED = 'sync.completed';
export const EVENT_RECOVERY_REQUIRED = 'recovery.required';

export interface ScanProgressEvent {
  taskId: string;
  seq: number;
  scannedDirs: number;
  discoveredSkills: number;
  invalidCount: number;
  currentPath: string | null;
  done: boolean;
}

export interface SyncProgressEvent {
  taskId: string;
  seq: number;
  phase: 'staging' | 'verifying' | 'backup' | 'committing' | 'cleanup' | 'item_done';
  itemIndex: number;
  itemCount: number;
  itemId: string | null;
  skillName: string | null;
  bytesDone: number | null;
  bytesTotal: number | null;
  message: string | null;
}

export interface SyncCompletedEvent {
  taskId: string;
  seq: number;
  status: TaskStatus;
  counts: TaskCounts;
  durationMs: number;
}

export interface RecoveryRequiredEvent {
  taskId: string | null;
  seq: number;
  transactionId: string;
  targetPath: string;
  skillName: string | null;
  reason: string;
  recoverable: 'auto' | 'manual';
}

export interface ContractEventMap {
  [EVENT_SCAN_PROGRESS]: ScanProgressEvent;
  [EVENT_SYNC_PROGRESS]: SyncProgressEvent;
  [EVENT_SYNC_COMPLETED]: SyncCompletedEvent;
  [EVENT_RECOVERY_REQUIRED]: RecoveryRequiredEvent;
}

// ---------------------------------------------------------------------------
// 命令签名表（invoke 参数与返回值的类型映射）
// ---------------------------------------------------------------------------

export interface ContractCommands {
  register_library: {
    args: { input: RegisterLibraryInput };
    result: RegisterLibraryResult;
  };
  select_library_root: {
    args: { libraryId: string; root: string };
    result: RegisterLibraryResult;
  };
  list_libraries: { args: Record<string, never>; result: LibrarySummary[] };
  set_library_pinned: { args: { libraryId: string; pinned: boolean }; result: null };
  remove_library: { args: { libraryId: string }; result: null };
  update_library_settings: {
    args: { libraryId: string; ignorePatterns: string[] };
    result: { configVersion: number };
  };
  scan_library: { args: { libraryId: string }; result: { taskId: string } };
  get_library: { args: { libraryId: string }; result: LibraryDetail };

  list_adapters: { args: { projectRoot?: string }; result: AdapterInfo[] };
  save_target: { args: { input: SaveTargetInput }; result: SaveTargetResult };
  list_targets: { args: Record<string, never>; result: TargetView[] };
  remove_target: { args: { targetId: string }; result: null };
  update_mappings: {
    args: { libraryId: string; skillId: string; physicalTargetIds: string[] };
    result: { mappings: MappingView[] };
  };

  create_sync_plan: { args: { input: CreateSyncPlanInput }; result: SyncPlanView };
  resolve_conflict: {
    args: { planId: string; planVersion: number; itemId: string; choice: ConflictChoice };
    result: SyncPlanView;
  };
  execute_sync_plan: {
    args: { planId: string; planVersion: number };
    result: { taskId: string };
  };
  cancel_task: {
    args: { taskId: string };
    result: { taskId: string; state: 'requested' | 'not_cancellable' };
  };
  get_task_snapshot: { args: { taskId: string }; result: TaskSnapshot };
  list_tasks: { args: { activeOnly: boolean }; result: TaskSnapshot[] };

  list_history: { args: { query: HistoryQuery }; result: HistoryPage };
  get_task_detail: { args: { taskId: string }; result: TaskSnapshot };
  create_restore_plan: { args: { input: CreateRestorePlanInput }; result: SyncPlanView };
  get_backup_stats: { args: Record<string, never>; result: BackupStats };
  list_snapshots: { args: { mappingId: string }; result: SnapshotView[] };
  update_backup_settings: {
    args: { retentionDays?: number; softCapBytes?: number };
    result: BackupStats;
  };

  open_registered_path: {
    args: { kind: 'library' | 'target' | 'task_item' | 'snapshot'; id: string };
    result: null;
  };
}

export type ContractCommandName = keyof ContractCommands;

// ---------------------------------------------------------------------------
// 类型安全的 invoke / listen 封装（需要 @tauri-apps/api；不需要可只取上方类型）
// ---------------------------------------------------------------------------

import { invoke } from '@tauri-apps/api/core';
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';

/** 类型安全调用后端命令；失败时 reject AppError */
export function call<N extends ContractCommandName>(
  name: N,
  ...args: keyof ContractCommands[N]['args'] extends never
    ? []
    : [ContractCommands[N]['args']]
): Promise<ContractCommands[N]['result']> {
  return invoke(name, args[0] ?? {}) as Promise<ContractCommands[N]['result']>;
}

/** 订阅后端事件（含 taskId 与递增 seq；重连后请先 get_task_snapshot 对齐） */
export function onContractEvent<N extends keyof ContractEventMap>(
  name: N,
  handler: (payload: ContractEventMap[N]) => void,
): Promise<UnlistenFn> {
  return listen<ContractEventMap[N]>(name, (e: Event<ContractEventMap[N]>) => handler(e.payload));
}
