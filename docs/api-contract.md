# SkillDock 前后端接口约定（API Contract）

文档版本：1.4 · 编写日期：2026-09-08 · 维护方：后端会话（src-tauri）
配套文件：[`src/lib/backend-contract.ts`](../src/lib/backend-contract.ts)（前端 TypeScript 类型，与本文同步维护）

> 接口有任何变化，后端会话必须先更新这两个文件并通知前端会话，再落地实现。
> 权威需求依据：`docs/SkillDock-需求与技术设计.md` §10.4（命令表）、§7（功能细则）、§8（同步语义）、§11（数据存储）。

**变更记录**
- 1.4（2026-09-09）：新增 `resolve_conflicts_bulk`（批量冲突解决，单次版本递增）；`open_registered_path` 的 kind 新增 `log_dir`；操作日志落盘到「文档\SkillDock\SkillDock-操作日志.jsonl」（append-only JSONL，见 §4.4 末注）。
- 1.3（2026-09-08）：`SourceCandidate.origin` 新增 `codex_skills`（候选发现补充 .codex/skills 位置）。新增枚举值，非破坏变更。
- 1.2（2026-09-08）：事件名改为 `scan://progress` 等冒号形式——Tauri 2 事件名不允许点号（仅字母数字、- / : _）。前端订阅常量值不变（仍用 backend-contract.ts 导出的常量）。
- 1.1（2026-09-08）：`ConflictInfo.kind` 新增 `same_content`（已有相同内容，可接管）与 `target_deleted`（目标已删除，可重装）；`ConflictChoice` 新增 `remove_with_backup`（移除计划中的「备份当前内容后移除」，§8.3）。均为新增枚举值，非破坏变更。
- 1.0（2026-09-08）：首版。

---

## 1. 通用约定

| 约定 | 说明 |
| --- | --- |
| 调用方式 | 前端通过 `invoke('<command>', args)` 调用；命令名为 `snake_case` |
| 字段命名 | 所有载荷字段一律 `camelCase`（Rust 侧 `#[serde(rename_all = "camelCase")]`） |
| 枚举值 | 所有枚举/判别联合的字符串值一律 `snake_case` |
| ID | 所有 ID 为字符串（UUID v4），由后端生成；前端不构造 ID |
| 时间 | ISO 8601 字符串，UTC（如 `2026-09-08T02:31:08.123Z`） |
| 字节数 | `number`（u64，序列化为 JSON 数字） |
| 路径 | 绝对路径字符串（Windows 原生形式，如 `C:\Users\x\.claude\skills`）；前端不得拼接路径交给执行类命令 |
| 空值 | 可选/可空字段不存在语义时用 `null`；数组不用 null |
| 错误 | 所有命令统一返回 `Result<T, AppError>`，见 §2 |

**路径授权原则（§10.4/§10.1）**：执行类命令（`execute_sync_plan`、`create_restore_plan`、`open_registered_path`）只接受后端登记对象的 ID 与计划 ID，**不接受任意「源路径 + 目标路径 + 覆盖开关」**。实际路径一律由后端根据登记记录与计划解析，并独立校验。

---

## 2. 错误模型 `AppError`

所有命令失败时 reject 一个结构化对象：

```ts
interface AppError {
  /** 机器可读错误码，见下表 */
  code: ErrorCode;
  /** 用户可读的简体中文消息：包含受影响技能、目标路径、实际原因与下一步建议 */
  message: string;
  /** 操作上下文（技能名、目标路径、计划 ID 等），随错误码而变；可为 null */
  context: Record<string, unknown> | null;
  /** 是否值得重试（如文件占用、临时 IO 错误）；false 表示需用户先处理 */
  retryable: boolean;
  /** 诊断 ID，对应后端日志条目，供「诊断详情」展示 */
  diagnosticId: string;
}

type ErrorCode =
  | 'permission_denied'   // 权限不足
  | 'file_locked'         // 文件被其他进程占用
  | 'disk_full'           // 磁盘/备份空间不足
  | 'plan_stale'          // 计划失效（预览后源/目标/配置已变化），需重新预览
  | 'invalid_path'        // 非法路径（越界、保留名、尾点/尾空格、设备路径、重叠根等）
  | 'target_drift'        // 目标漂移（提交前复核发现目标已变）
  | 'backup_failed'       // 备份失败（该项不得执行覆盖）
  | 'recovery_pending'    // 存在待恢复事务，该目标暂停新任务
  | 'not_found'           // 引用的库/技能/目标/计划/任务不存在
  | 'validation_failed'   // 输入或 Skill 元数据校验失败
  | 'unsupported'         // 不支持的对象（符号链接、外部引用、UNC/网络盘等）
  | 'path_overlap'        // 源根与目标根相同/互为父子/多目标互相包含
  | 'conflict_unresolved' // 仍存在未处理的冲突项，不能执行
  | 'io_error'            // 其他文件系统错误
  | 'internal';           // 未预期内部错误（诊断详情见日志）
```

**前端约定**：展示 `message`；`code` 用于决定引导动作（如 `plan_stale` → 提示重新预览；`recovery_pending` → 引导到恢复界面）；`context` 折叠进「诊断详情」。

---

## 3. 核心枚举

```ts
/** 矩阵单元格状态（§8.2 决策表的展示态） */
type MatrixCellState =
  | 'no_mapping'              // 未选择该目标
  | 'to_add'                  // 待新增（无托管，S 存在，T = ∅）
  | 'same_content_unmanaged'  // 已有相同内容（无托管，S = T），可显式接管
  | 'unmanaged_conflict'      // 同名冲突（无托管，S ≠ T 且 T 存在），默认保留
  | 'synced'                  // 已同步（S = T = B）
  | 'source_updated'          // 源已更新（S ≠ B，T = B），待更新
  | 'target_modified'         // 目标已修改（S = B，T ≠ B），冲突
  | 'both_modified'          // 双方已修改（S ≠ B，T ≠ B，S ≠ T），冲突
  | 'aligned_externally'      // 双方内容已一致（S = T ≠ B），核验后刷新基线
  | 'target_deleted'          // 目标已删除（S 存在，T = ∅），默认保留删除状态
  | 'source_removed'          // 源已移除（S = ∅，T 存在），目标保留待处理
  | 'ownership_conflict'      // 来源冲突（目标名已归属其他源库）
  | 'invalid'                 // Skill 校验失败，阻塞
  | 'unsupported'             // 不支持（符号链接、外部引用等），阻塞
  | 'paused';                 // 映射已暂停

/** Skill 校验状态（§4.2） */
type ValidationStatus = 'valid' | 'invalid' | 'unsupported';

/** 计划项动作 */
type PlanAction =
  | 'create'      // 新增：复制并登记托管
  | 'update'      // 更新：覆盖目标（含托管范围内的文件删除）
  | 'adopt'       // 接管：内容一致，只建立基线，不复制
  | 'overwrite'   // 冲突解决后用源覆盖（先备份）
  | 'take_over'   // 接管非托管同名目录并用源覆盖（先备份）
  | 'reinstall'   // 目标已删除后重新安装
  | 'remove'      // 移除托管技能（独立操作，先备份）
  | 'restore'     // 恢复历史版本（先备份当前）
  | 'skip'        // 跳过（默认不纳入：冲突未处理、无变化等）
  | 'blocked';    // 阻塞（校验失败、路径非法、空间不足等）

/** 冲突选择（resolve_conflict 的 choice；§6.3） */
type ConflictChoice =
  | 'keep_target'           // 本次保留目标
  | 'overwrite_with_source' // 备份后用源覆盖（恢复计划中表示「确认恢复覆盖」）
  | 'adopt_existing'        // 接管现有目录（内容相同，仅建基线）
  | 'take_over'             // 接管并用源覆盖（非托管同名目录）
  | 'pause_mapping'         // 始终暂停此映射
  | 'reinstall'             // 目标已删除：手动选择重新安装
  | 'keep_deleted'          // 目标已删除：维持删除状态
  | 'transfer_ownership'    // 来源冲突：明确转移归属
  | 'remove_with_backup';   // 移除计划：看过差异后备份当前内容再移除（§8.3）

/** 任务状态（§11.1） */
type TaskStatus =
  | 'queued' | 'running'                        // 进行中
  | 'completed' | 'partial'                     // 全部完成 / 部分完成
  | 'failed' | 'cancelled' | 'recovery_pending'; // 失败 / 已取消 / 待恢复

/** 单项结果状态：跳过与冲突单独统计，不混为成功 */
type TaskItemStatus =
  | 'pending' | 'running'
  | 'success' | 'failed' | 'skipped' | 'conflict_pending' | 'cancelled' | 'recovery_pending';

type TaskKind = 'scan' | 'sync' | 'remove' | 'restore';
type TaskTrigger = 'manual' | 'retry' | 'restore' | 'startup_recovery';
type PlanOperation = 'sync' | 'remove' | 'restore';
type PlanStatus = 'active' | 'stale' | 'executed' | 'cancelled';
type TargetScope = 'user' | 'project' | 'custom';
type Availability = 'exists' | 'will_create' | 'no_permission' | 'invalid_path' | 'unsupported_location';
```

---

## 4. 命令清单

> **调用时序（前端必读）**：技能列表来自后端数据库，数据库由 `scan_library` 填充。
>
> 1. 登记：`register_library` → 若返回 `needsRootChoice = true`，用户选择后调 `select_library_root`。
> 2. 扫描：源根确定后**必须**调用 `scan_library`（后台任务；进度走 `scan://progress`，终态走 `sync://completed`）。未扫描的库，`get_library` 永远返回空技能列表。
> 3. 读取：`get_library` 返回矩阵数据。
> 4. 日常刷新（F5、启动恢复上次库、同步完成后）：先 `scan_library` 再 `get_library`；`get_library` 本身不重扫磁盘。
> 5. 预览与执行：`create_sync_plan` →（如有冲突）`resolve_conflict` → `execute_sync_plan` → 事件流 + `get_task_snapshot`。
> 6. 单技能目录：`mode: 'auto'` 登记的目录本身含 SKILL.md 但其直接子目录都不是技能时，返回 `sourceRoot = null`；请用 `mode: 'single'` 重新登记该目录。

### 4.1 技能库

#### `register_library` — 登记技能库（§10.4）

```ts
invoke<RegisterLibraryResult>('register_library', { input })

interface RegisterLibraryInput {
  path: string;                       // 用户选择的目录（选择器或拖入）
  displayName?: string;               // 缺省取目录名
  mode?: 'auto' | 'collection' | 'single'; // 缺省 auto；single = 添加单个技能（目录本身即一个 Skill）
  selectedRoot?: string;              // 多候选时用户选定的源根；缺省按 §4.1 规则自动/返回候选
}

interface RegisterLibraryResult {
  libraryId: string;
  canonicalPath: string;
  sourceRoot: string | null;          // 已确定的源根；needsRootChoice 时为 null
  needsRootChoice: boolean;           // 多个有效候选，待用户选择
  candidates: SourceCandidate[];      // 候选发现结果（§4.1 已知位置浅层发现）
  diagnostics: PathDiagnostic[];      // 路径诊断（不可用原因、不支持的卷类型等）
  recentLibraries: LibrarySummary[];  // 登记后的最近列表（含本项）
}

interface SourceCandidate {
  path: string;                       // 候选源根绝对路径
  origin: 'root' | 'skills' | 'agents_skills' | 'claude_skills' | 'codex_skills' | 'custom';
  validSkillCount: number;
  invalidSkillCount: number;
}

interface PathDiagnostic {
  level: 'info' | 'warning' | 'error';
  code: string;                       // 如 'unc_path' / 'cloud_placeholder' / 'overlap_with_target'
  message: string;
}
```

规则：相同真实目录（大小写折叠 + 规范化后）只登记一次，重复登记返回既有 `libraryId`；不自动合并不同集合。

#### `select_library_root` — 多候选时选定源根

```ts
invoke<RegisterLibraryResult>('select_library_root', { libraryId, root })
// root 必须属于该库已发现的 candidates，否则 validation_failed
```

#### `list_libraries` — 最近打开（FR-01：最近 10 个、可固定）

```ts
invoke<LibrarySummary[]>('list_libraries')

interface LibrarySummary {
  libraryId: string;
  displayName: string;
  canonicalPath: string;
  sourceRoot: string | null;
  pinned: boolean;
  lastOpenedAt: string | null;
  pathValid: boolean;                 // 路径失效时可重新定位
  skillCount: number;
  invalidCount: number;
}
```

#### `set_library_pinned` / `remove_library`

```ts
invoke<null>('set_library_pinned', { libraryId, pinned })
invoke<null>('remove_library', { libraryId })
// remove：从最近列表移除并停用其映射；历史归档保留；不触碰源文件与既有托管目录（§7.1）
```

#### `update_library_settings` — 源库级忽略模式（§4.3）

```ts
invoke<{ configVersion: number }>('update_library_settings', { libraryId, ignorePatterns })
// ignorePatterns: string[]，相对源根路径的 glob 语义
// 保存后 configVersion 递增 → 所有基于旧配置的计划失效（plan_stale）
```

#### `scan_library` — 扫描（§10.4，后台任务）

```ts
invoke<{ taskId: string }>('scan_library', { libraryId })
// 进度经 scan://progress 事件分批推送；完成后经 get_task_snapshot 或 get_library 取结果
```

#### `get_library` — 库详情（矩阵数据源）

```ts
invoke<LibraryDetail>('get_library', { libraryId })

interface LibraryDetail {
  summary: LibrarySummary;
  ignorePatterns: string[];
  configVersion: number;
  skills: SkillView[];
}

interface SkillView {
  skillId: string;                    // 库 ID + 相对路径 的内部身份（§4.2）
  relPath: string;
  name: string | null;                // 无效项可为 null
  description: string | null;
  fileCount: number;
  totalBytes: number;
  lastContentChangedAt: string | null;
  digest: string | null;              // 技能摘要（§8.1）
  validation: {
    status: ValidationStatus;
    issues: ValidationIssue[];        // 位置与原因，如「SKILL.md 第 4 行 YAML 格式错误」
  };
  targets: Record<string, MatrixCell>; // key = physicalTargetId，仅含已配置目标
}

interface ValidationIssue {
  code: string;                       // missing_name / bad_name / name_dir_mismatch / missing_description
                                      // yaml_error / duplicate_name / unreadable / symlink / external_reference
  message: string;
  line: number | null;
  column: number | null;
}

interface MatrixCell {
  state: MatrixCellState;
  mappingId: string | null;
  targetDirName: string | null;
  detail: string | null;              // 状态补充说明（如「此位置也可能被 Cursor 读取」）
}
```

---

### 4.2 目标管理

#### `list_adapters` — 适配器卡片数据（§5、§5.1）

```ts
invoke<AdapterInfo[]>('list_adapters', { projectRoot? })
// projectRoot 提供时，同时解析项目级模板并检测

interface AdapterInfo {
  adapterId: 'codex' | 'claude_code' | 'cursor' | 'copilot_vscode' | 'custom';
  displayName: string;
  adapterVersion: string;
  verifiedAt: string;                 // 规则核实日期（非真实加载验证；§5 边界）
  docsUrl: string;
  scopes: TargetScope[];              // 支持的作用域
  userTemplate: string | null;        // 如 %USERPROFILE%\.claude\skills
  projectTemplate: string | null;
  resolved: {
    user: ResolvedPath | null;
    project: ResolvedPath | null;     // 未提供 projectRoot 时为 null
  };
  detectedHints: string[];            // 「发现了应用或配置线索」的依据（仅痕迹，不代表已验证加载）
  alsoReadBy: string[];               // 共享读取提示（§5：如 .claude 也被 Cursor/Copilot 读取）
  notes: string[];                    // 边界说明（如 .codex 旧路径需按当前版本确认）
}

interface ResolvedPath {
  path: string;
  source: string;                     // 解析来源说明（模板/环境变量/用户输入）
  availability: Availability;
  detail: string | null;              // 无权限/将创建/不支持位置的说明
}
```

#### `save_target` — 保存目标（§10.4）

```ts
invoke<SaveTargetResult>('save_target', { input })

interface SaveTargetInput {
  adapterId: string;
  scope: TargetScope;
  path?: string;                      // 覆盖模板解析结果（自定义目录必填）
  projectRoot?: string;               // scope = project 必填
  displayName?: string;               // 自定义目录必填
}

interface SaveTargetResult {
  targetId: string;
  physicalTargetId: string;           // 同一真实目录复用同一 ID（§5.2.3）
  resolvedPath: string;
  resolutionSource: string;
  availability: Availability;
  sharedWithTools: string[];          // 关联工具（同一物理目标的其它适配器）
  warnings: string[];                 // 如「此位置也可能被 Cursor / Copilot 读取」
}
```

校验（§5.2）：源根与目标根相同/互为父子、多目标根互相包含 → `path_overlap`；UNC/网络盘/WSL → `unsupported_location`；目标目录不存在可保存（`will_create`），首次执行前预览显示将创建的完整路径。

#### `list_targets` / `remove_target`

```ts
invoke<TargetView[]>('list_targets')

interface TargetView {
  targetId: string;
  physicalTargetId: string;
  adapterId: string;
  displayName: string;
  scope: TargetScope;
  resolvedPath: string;
  resolutionSource: string;
  availability: Availability;
  sharedWithTools: string[];
  mappedSkillCount: number;           // 关联技能数
  enabled: boolean;
}

invoke<null>('remove_target', { targetId })
// 停用相关映射；不删除磁盘内容（§7.1/§8.3 停止管理只撤销记录）
```

#### `update_mappings` — 按技能设置目标（FR-04）

```ts
invoke<{ mappings: MappingView[] }>('update_mappings', {
  libraryId, skillId, physicalTargetIds   // 全量替换该技能的目标集合；[] = 取消全部勾选
})

interface MappingView {
  mappingId: string;
  skillId: string;
  physicalTargetId: string;
  targetDirName: string;              // = 合法目录名（§4.1）
  enabled: boolean;
  pausedReason: string | null;
}
```

约束：「物理目标 + 目标目录名」在活动映射中唯一（§11.1）；与他库归属冲突时返回 `conflict_unresolved` + `ownership_conflict` 上下文，不后台抢占（AC-16）。

---

### 4.3 同步计划与执行

#### `create_sync_plan` — 生成预览（§10.4、§7.2）

```ts
invoke<SyncPlanView>('create_sync_plan', { input })

interface CreateSyncPlanInput {
  libraryId: string;
  operation: PlanOperation;           // sync（默认）| remove（移除托管技能，§8.3 独立操作）
  mappingIds: string[];               // 本次纳入的映射；remove 时必须非空且逐项列出
}

interface SyncPlanView {
  planId: string;
  planVersion: number;                // resolve_conflict 后递增
  libraryId: string;
  operation: PlanOperation;
  status: PlanStatus;
  createdAt: string;
  configVersion: number;              // 生成时的库配置版本
  groups: PlanGroupView[];            // 按物理目标分组（§7.2）
  summary: PlanSummary;
}

interface PlanGroupView {
  physicalTargetId: string;
  targetPath: string;
  sharedWithTools: string[];
  items: PlanItemView[];
}

interface PlanItemView {
  itemId: string;
  mappingId: string;
  skillId: string;
  skillName: string;
  relPath: string;
  action: PlanAction;
  fileChanges: FileChange[];          // 逐项文件变化；文件删除必须可展开（§7.2）
  excludedByIgnore: { relPath: string }[]; // 被忽略规则排除的文件（§4.3）
  needsBackup: boolean;
  targetPath: string;                 // 该技能的目标完整路径
  stateBefore: MatrixCellState;       // 生成计划时的判定来源
  conflict: ConflictInfo | null;      // action 为冲突类时非空
  blockedReason: string | null;       // action = blocked 时的原因
  selected: boolean;                  // 默认：无冲突/无阻塞项 = true，其余 = false
  decision: ConflictChoice | null;    // resolve_conflict 后的选择
}

interface FileChange {
  kind: 'add' | 'update' | 'delete';
  relPath: string;
  bytes: number;
  sourceDigest: string | null;
  targetDigest: string | null;
  diffPreviewable: boolean;           // §7.2：>1MiB / >20000 行 / 非 UTF-8 / 二进制 = false
}

interface ConflictInfo {
  kind: 'unmanaged_same_name' // 非托管同名、内容不同 → [keep_target, take_over]
      | 'same_content'        // 非托管同名、内容一致 → [adopt_existing, keep_target]
      | 'target_modified'     // 目标漂移（含移除计划的漂移）→ [keep_target, overwrite_with_source / remove_with_backup, pause_mapping]
      | 'both_modified'       // 双方已修改 → [keep_target, overwrite_with_source, pause_mapping]
      | 'target_deleted'      // 目标已删除 → [keep_deleted, reinstall]
      | 'ownership'           // 来源冲突 → [transfer_ownership, keep_target]
      | 'restore_drift';      // 恢复点之后又有修改 → [keep_target, overwrite_with_source]
  message: string;
  availableChoices: ConflictChoice[]; // 后端按冲突类型给出可选动作
}

interface PlanSummary {
  createCount: number;
  updateCount: number;
  removeCount: number;
  restoreCount: number;
  skipCount: number;
  conflictCount: number;
  blockedCount: number;
  executableCount: number;            // 仅无冲突、无阻塞、已选中的项；与主按钮数字一致（§7.2）
  totalBytes: number;
}
```

规则：计划保存生成时的源摘要、目标摘要、配置指纹；文件或配置变化后执行返回 `plan_stale`，前端提示「内容已变化，请刷新预览」。展示层设置（排序、主题）不影响指纹。

#### `resolve_conflict` — 冲突选择（§10.4）

```ts
invoke<SyncPlanView>('resolve_conflict', { planId, planVersion, itemId, choice })
// 返回新版本的完整计划视图（planVersion 递增）；版本不匹配 → plan_stale
// choice 必须属于该项 conflict.availableChoices，否则 validation_failed
// 「接管/覆盖/转移归属」只对当前内容与当前计划有效（§8.2）
```

#### `resolve_conflicts_bulk` — 批量冲突解决（§6.3 批量场景）

```ts
invoke<{ plan: SyncPlanView; outcome: BulkResolveOutcome }>('resolve_conflicts_bulk', {
  planId, planVersion, kinds, choice,
})
// kinds：要批量处理的 ConflictInfo.kind 列表，如 ['same_content'] 或 ['unmanaged_same_name']
// choice：对这些项统一应用的选择；某项的 availableChoices 不含该选择时跳过（计入 skipped）
// 整批只递增一次 planVersion；kinds 匹配但无可执行项时 applied 为空数组

interface BulkResolveOutcome {
  applied: string[];   // 已应用的 itemId
  skipped: string[];   // 类型匹配但选择不适用的 itemId
}
```

#### `execute_sync_plan` — 执行（§10.4、§7.3）

```ts
invoke<{ taskId: string }>('execute_sync_plan', { planId, planVersion })
// 执行前重新验证全部项的源/目标摘要与配置版本；不一致 → plan_stale，原计划失效
// 仍存在未处理冲突项（选中但无 decision）→ conflict_unresolved
```

执行语义（后端保证，前端只需正确展示）：

- 无变化技能不写入、不刷新时间戳、不制造成功备份（AC-07）。
- 同一物理目标串行提交；不同物理目标最多 2 个并行。
- 单项失败保留其原版本或进入恢复状态，其他项继续；目标级共性错误暂停该目标余下操作。
- 更新/覆盖/移除/恢复前必须备份；备份失败该项不执行（`backup_failed`，AC-19）。
- 每技能事务：暂存 → 校验 → 备份登记 → 复核摘要 → 改名替换 → 核验 → 提交基线（§8.4）。

#### `cancel_task` — 取消（§10.4、§7.3）

```ts
invoke<{ taskId: string; state: 'requested' | 'not_cancellable' }>('cancel_task', { taskId })
// 未开始项取消；正在提交的单元先完成或回滚再停
```

#### `get_task_snapshot` / `list_tasks` — 任务快照（重连恢复，§10.4）

```ts
invoke<TaskSnapshot>('get_task_snapshot', { taskId })
invoke<TaskSnapshot[]>('list_tasks', { activeOnly })

interface TaskSnapshot {
  taskId: string;
  kind: TaskKind;
  trigger: TaskTrigger;
  status: TaskStatus;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  counts: { success: number; failed: number; skipped: number; conflictPending: number; cancelled: number; total: number };
  items: TaskItemView[];
  error: AppError | null;
}

interface TaskItemView {
  itemId: string;
  mappingId: string | null;
  skillName: string;
  targetPath: string;
  action: PlanAction;
  status: TaskItemStatus;
  error: AppError | null;
  bytesProcessed: number;
  bytesTotal: number;
  snapshotId: string | null;          // 产生的备份快照 ID（历史恢复入口）
}
```

---

### 4.4 历史与恢复

#### `list_history` — 历史列表（§10.4、§7.4）

```ts
invoke<HistoryPage>('list_history', { query })

interface HistoryQuery {
  cursor?: string;
  limit?: number;                     // 缺省 50，最大 200
  libraryId?: string;
  targetId?: string;
  status?: TaskStatus;
}

interface HistoryPage {
  items: HistoryTaskSummary[];
  nextCursor: string | null;
}

interface HistoryTaskSummary {
  taskId: string;
  kind: TaskKind;
  trigger: TaskTrigger;
  startedAt: string | null;
  durationMs: number | null;
  status: TaskStatus;
  counts: TaskSnapshot['counts'];
  libraryName: string | null;
  targetPaths: string[];
}
```

#### `get_task_detail` — 历史详情（文件清单、恢复入口、可读错误）

```ts
invoke<TaskSnapshot>('get_task_detail', { taskId })
// 同 TaskSnapshot；每项 snapshotId 非空且快照可用时可恢复
```

#### `create_restore_plan` — 生成恢复计划（§10.4、§6.5、§7.4）

```ts
invoke<SyncPlanView>('create_restore_plan', { input })

interface CreateRestorePlanInput {
  taskId: string;                     // 历史任务
  itemIds: string[];                  // 其中要恢复的项
}

// 返回 operation = 'restore' 的 SyncPlanView；规则：
// - 恢复快照缺失/损坏 → 项为 blocked（blockedReason = '备份已过期' 等），不将当前目标删除（AC-24）
// - 当前目标相对恢复点之后又被修改 → 项为冲突（kind = 'restore_drift'），
//   用户经 resolve_conflict 选 overwrite_with_source（此处含义=确认恢复覆盖）后才可执行（AC-23）
// - 恢复执行前对当前目标再次快照；成功后将实际恢复结果登记为新基线；
//   恢复为「原先不存在」时删除本应用创建且未被再次修改的目标并记录删除状态（AC-22）
// - 恢复后相关映射暂停自动同步（pausedReason = 'restored'），源库不回退
```

恢复计划同样经 `execute_sync_plan` 执行，走 §8.4 事务流程。

> **操作日志（独立于数据库）**：每个任务（扫描/同步/移除/恢复）完成时，后端向「文档\SkillDock\SkillDock-操作日志.jsonl」追加一行 JSON（时间、任务、状态、计数、逐项结果）。该文件是给用户的长期可读记录，数据库损坏或重装不受影响；前端可在设置页提供「打开日志目录」入口（`open_registered_path` kind = `log_dir`）。

#### `get_backup_stats` / `list_snapshots`

```ts
invoke<BackupStats>('get_backup_stats')

interface BackupStats {
  totalBytes: number;                 // 已用
  reclaimableBytes: number;           // 可释放（超保留期且未保护）
  protectedBytes: number;             // 不能清理（未完成事务/固定/每映射最近一次）
  retentionDays: number;              // 默认 30
  softCapBytes: number;               // 默认 2 GiB
}

invoke<SnapshotView[]>('list_snapshots', { mappingId })

interface SnapshotView {
  snapshotId: string;
  taskItemId: string | null;
  mappingId: string | null;
  targetPath: string;
  existedBefore: boolean;             // false = 「原先不存在」记录
  digest: string | null;
  bytes: number;
  pinned: boolean;
  available: boolean;                 // 快照完整可恢复；false 时恢复入口显示「备份已过期」
  createdAt: string;
}
```

#### `update_backup_settings`

```ts
invoke<BackupStats>('update_backup_settings', { retentionDays?, softCapBytes? })
```

---

### 4.5 受信任位置打开

#### `open_registered_path`（§10.4）

```ts
invoke<null>('open_registered_path', { kind, id })
// kind: 'library' | 'target' | 'task_item' | 'snapshot' | 'log_dir'（打开操作日志目录，id 传空串）
// 仅打开后端登记过的真实路径（经 Windows 文件管理器，不拼接 Shell 命令）；
// 路径失效返回结构化错误（invalid_path / not_found）
```

---

## 5. 事件（§10.4）

事件经 Tauri `emit` 广播，前端 `listen('<event>', cb)` 订阅。所有事件载荷含 `taskId` 与**递增序号 `seq`**（每次应用运行内单调递增，全局统一序列）；前端重连后用 `get_task_snapshot` 对齐，再按 `seq` 续接事件流。

```ts
// 扫描进度（scan_library）
interface ScanProgressEvent {
  taskId: string; seq: number;
  scannedDirs: number;
  discoveredSkills: number;
  invalidCount: number;
  currentPath: string | null;
  done: boolean;
}

// 同步进度（execute_sync_plan；按阶段、技能数与字节数反馈，不编造百分比）
interface SyncProgressEvent {
  taskId: string; seq: number;
  phase: 'staging' | 'verifying' | 'backup' | 'committing' | 'cleanup' | 'item_done';
  itemIndex: number;                  // 当前项序号（0 起）
  itemCount: number;
  itemId: string | null;
  skillName: string | null;
  bytesDone: number | null;           // 无法估算时为 null
  bytesTotal: number | null;
  message: string | null;
}

// 任务终态（sync / remove / restore / scan 均广播）
interface SyncCompletedEvent {
  taskId: string; seq: number;
  status: TaskStatus;                 // completed / partial / failed / cancelled / recovery_pending
  counts: TaskSnapshot['counts'];
  durationMs: number;
}

// 启动或执行中发现待恢复事务（§8.4.6）
interface RecoveryRequiredEvent {
  taskId: string | null; seq: number;
  transactionId: string;
  targetPath: string;
  skillName: string | null;
  reason: string;                     // 如「提交中断，旧版本已保留」/「发现未知第三方内容，需人工处理」
  recoverable: 'auto' | 'manual';     // manual = 保留现场，需用户在恢复界面处理
}
```

事件名常量：`scan://progress`、`sync://progress`、`sync://completed`、`recovery://required`。

---

## 6. 契约级不变量（后端保证，前端可依赖）

1. **不静默覆盖非托管内容**：`unmanaged_conflict` 项默认 `selected = false`；未经 `resolve_conflict` 选择 `take_over`，执行不包含该项。
2. **不自动删除源已移除的整项技能**：`source_removed` 项不出现在 `sync` 计划的可执行项中；整项移除仅由 `operation = 'remove'` 的独立计划产生。
3. **预览后内容变化必须重新生成计划**：执行前逐项复核源/目标摘要与配置版本，任一不一致返回 `plan_stale`。
4. **备份失败不得继续覆盖**：备份失败的项以 `backup_failed` 失败，目标保持原内容。
5. **汇总一致性**：`summary.executableCount` 只统计无冲突、无阻塞且选中的项；冲突/阻塞/跳过不计入（§7.2）。
6. **跳过与冲突不是成功**：任务计数中 `skipped`、`conflictPending` 与 `success` 分列（§11.1）。
7. **唯一归属**：同一物理目标同一目录名在同一时刻至多一个活动映射拥有者；`ownership_conflict` 不自动抢占（AC-16）。
8. **共享物理目标只写一次**：同一物理目标被多个工具引用时，计划合并为一个分组，只执行一次复制与一次备份（AC-15）。

---

## 7. 版本与变更流程

- 本文件与 `backend-contract.ts` 同步修改，版本号一致。
- 破坏性变更（字段删除/改名、语义变化）必须先在 issue 跟踪器通知前端会话。
- 新增命令/字段视为非破坏变更，更新文件并通知即可。
