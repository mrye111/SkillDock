-- SkillDock 初始 schema（需求 §11.1 实体）
-- 身份原则：Skill 以「库 ID + 相对路径」识别；活动映射以「物理目标 ID + 目标目录名」唯一。

CREATE TABLE libraries (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  canonical_path TEXT NOT NULL,          -- 用户选择的目录（规范化后）
  source_root TEXT,                      -- 实际扫描根；多候选待选择时为 NULL
  dir_identity TEXT,                     -- 真实目录身份（规范化 + 大小写折叠）
  scan_mode TEXT NOT NULL DEFAULT 'collection',  -- collection | single
  skill_filter TEXT,                     -- single 模式：JSON 数组，仅扫描这些相对路径
  ignore_patterns TEXT NOT NULL DEFAULT '[]',    -- JSON 数组，相对源根的 glob
  config_version INTEGER NOT NULL DEFAULT 1,
  pinned INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  last_opened_at TEXT,
  created_at TEXT NOT NULL
);
-- 相同真实目录只登记一次（归档后允许重新登记）
CREATE UNIQUE INDEX ux_libraries_identity
  ON libraries(dir_identity) WHERE archived = 0 AND dir_identity IS NOT NULL;

CREATE TABLE skills (
  id TEXT PRIMARY KEY,
  library_id TEXT NOT NULL REFERENCES libraries(id),
  rel_path TEXT NOT NULL,                -- 相对 source_root
  name TEXT,
  description TEXT,
  file_count INTEGER NOT NULL DEFAULT 0,
  total_bytes INTEGER NOT NULL DEFAULT 0,
  digest TEXT,                           -- 技能摘要；校验失败为 NULL
  manifest_json TEXT NOT NULL DEFAULT '[]',
  excluded_json TEXT NOT NULL DEFAULT '[]',  -- 被忽略规则排除的文件
  validation_status TEXT NOT NULL,       -- valid | invalid | unsupported
  validation_json TEXT NOT NULL DEFAULT '[]',
  missing INTEGER NOT NULL DEFAULT 0,    -- 最近一次扫描时目录已不存在（源已移除）
  last_content_changed_at TEXT,
  last_scanned_at TEXT,
  UNIQUE(library_id, rel_path)
);

CREATE TABLE physical_targets (
  id TEXT PRIMARY KEY,
  canonical_path TEXT NOT NULL,          -- 展示与访问用
  identity TEXT NOT NULL UNIQUE,         -- 大小写折叠身份；同一真实目录只登记一次
  availability TEXT NOT NULL,            -- exists | will_create | no_permission | invalid_path | unsupported_location
  last_checked_at TEXT
);

CREATE TABLE targets (
  id TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL,              -- codex | claude_code | cursor | copilot_vscode | custom
  adapter_version TEXT NOT NULL,
  scope TEXT NOT NULL,                   -- user | project | custom
  display_name TEXT NOT NULL,
  path_template TEXT NOT NULL,           -- 解析前模板或自定义原样
  resolution_source TEXT NOT NULL,       -- 路径来源说明
  project_root TEXT,
  physical_target_id TEXT NOT NULL REFERENCES physical_targets(id),
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE mappings (
  id TEXT PRIMARY KEY,
  library_id TEXT NOT NULL REFERENCES libraries(id),
  skill_id TEXT NOT NULL REFERENCES skills(id),
  physical_target_id TEXT NOT NULL REFERENCES physical_targets(id),
  target_dir_name TEXT NOT NULL,         -- 目标目录名（= 合法技能目录名）
  enabled INTEGER NOT NULL DEFAULT 1,
  owner_library_id TEXT NOT NULL,        -- 归属库；唯一约束保证一个有效拥有者
  paused_reason TEXT,                    -- user | restored | source_removed | ...
  created_at TEXT NOT NULL
);
-- 对活动映射建立「物理目标 + 目标目录名」唯一约束（§11.1）
CREATE UNIQUE INDEX ux_mappings_active_owner
  ON mappings(physical_target_id, target_dir_name) WHERE enabled = 1;
CREATE INDEX ix_mappings_skill ON mappings(skill_id);
CREATE INDEX ix_mappings_library ON mappings(library_id);

CREATE TABLE baselines (
  mapping_id TEXT PRIMARY KEY REFERENCES mappings(id),
  digest TEXT,                           -- NULL = 「不存在」标记（历史恢复产生；§8.2）
  manifest_json TEXT NOT NULL DEFAULT '[]',
  task_id TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE sync_plans (
  id TEXT PRIMARY KEY,
  version INTEGER NOT NULL DEFAULT 1,
  library_id TEXT NOT NULL REFERENCES libraries(id),
  operation TEXT NOT NULL,               -- sync | remove | restore
  fingerprint TEXT NOT NULL,             -- 配置版本 + 逐项 S/T/B 摘要与动作/决策
  items_json TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active', -- active | stale | executed | cancelled
  created_at TEXT NOT NULL
);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,                    -- scan | sync | remove | restore
  trigger TEXT NOT NULL,                 -- manual | retry | restore | startup_recovery
  library_id TEXT,
  plan_id TEXT,
  status TEXT NOT NULL,                  -- queued | running | completed | partial | failed | cancelled | recovery_pending
  counts_json TEXT NOT NULL DEFAULT '{}',
  error_json TEXT,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT
);
CREATE INDEX ix_tasks_created ON tasks(created_at);

CREATE TABLE task_items (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  mapping_id TEXT,
  skill_name TEXT,
  target_path TEXT,
  action TEXT,
  status TEXT NOT NULL,                  -- pending | running | success | failed | skipped | conflict_pending | cancelled | recovery_pending
  error_json TEXT,
  bytes_processed INTEGER NOT NULL DEFAULT 0,
  bytes_total INTEGER NOT NULL DEFAULT 0,
  pre_digest TEXT,
  post_digest TEXT,
  seq INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_task_items_task ON task_items(task_id);

CREATE TABLE snapshots (
  id TEXT PRIMARY KEY,
  task_item_id TEXT REFERENCES task_items(id),
  mapping_id TEXT,
  target_path TEXT NOT NULL,
  existed_before INTEGER NOT NULL,       -- 0 = 首次创建记录「原先不存在」（§7.4）
  backup_dir TEXT,                       -- 相对 backups/ 的目录名；existed_before=0 时为 NULL
  digest TEXT,
  bytes INTEGER NOT NULL DEFAULT 0,
  pinned INTEGER NOT NULL DEFAULT 0,
  pruned_at TEXT,                        -- 已清理时间；历史保留但恢复入口标记过期
  created_at TEXT NOT NULL
);
CREATE INDEX ix_snapshots_mapping ON snapshots(mapping_id);

CREATE TABLE recovery_transactions (
  id TEXT PRIMARY KEY,                   -- = 事务 ID = journals/<id>.json
  task_item_id TEXT,
  mapping_id TEXT,
  physical_target_id TEXT,
  skill_dir_name TEXT,
  phase TEXT NOT NULL,                   -- staged | backed_up | old_moved_out | new_moved_in | committed | done | aborted | manual
  staging_path TEXT,
  old_path TEXT,
  target_path TEXT NOT NULL,
  backup_id TEXT,
  expected_source_digest TEXT,
  expected_old_digest TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
