//! SQLite 存储层（需求 §11）。Rust 侧唯一访问点；不向前端开放 SQL。
//!
//! - 迁移：版本化、启动时备份数据库后执行；失败恢复备份并报告（§11.2）
//! - 身份：Skill = 库 ID + 相对路径；活动映射 = 物理目标 + 目录名 唯一
//! - 数据库丢失语义由上层保证：既有目标一律按非托管目录处理（§11.2）

use crate::contract::*;
use crate::error::{AppError, AppResult};
use crate::scanner::ScannedSkill;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

const MIGRATIONS: &[&str] = &[include_str!("../../migrations/0001_init.sql")];

pub struct Store {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

impl Store {
    /// 连接 guard；Mutex 中毒时取回内部值（本地单进程数据库，中毒不代表数据损坏）。
    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Debug, Clone)]
pub struct LibraryRow {
    pub id: String,
    pub display_name: String,
    pub canonical_path: String,
    pub source_root: Option<String>,
    pub dir_identity: Option<String>,
    pub scan_mode: String,
    pub skill_filter: Option<Vec<String>>,
    pub ignore_patterns: Vec<String>,
    pub config_version: i64,
    pub pinned: bool,
    pub archived: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SkillRow {
    pub id: String,
    pub library_id: String,
    pub rel_path: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    pub digest: Option<String>,
    pub manifest: Vec<crate::scanner::FileEntry>,
    pub excluded: Vec<String>,
    pub validation_status: ValidationStatus,
    pub validation: Vec<ValidationIssue>,
    pub missing: bool,
    pub last_content_changed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PhysicalTargetRow {
    pub id: String,
    pub canonical_path: String,
    pub availability: Availability,
}

#[derive(Debug, Clone)]
pub struct TargetRow {
    pub id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub scope: TargetScope,
    pub display_name: String,
    pub path_template: String,
    pub resolution_source: String,
    pub project_root: Option<String>,
    pub physical_target_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct MappingRow {
    pub id: String,
    pub library_id: String,
    pub skill_id: String,
    pub physical_target_id: String,
    pub target_dir_name: String,
    pub enabled: bool,
    pub owner_library_id: String,
    pub paused_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BaselineRow {
    pub mapping_id: String,
    /// None = 「不存在」标记（§8.2：历史恢复产生的有托管记录的不存在）
    pub digest: Option<String>,
    pub manifest: Vec<crate::scanner::FileEntry>,
    pub task_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PlanRow {
    pub id: String,
    pub version: i64,
    pub library_id: String,
    pub operation: PlanOperation,
    pub fingerprint: String,
    pub items: Vec<PlanItem>,
    pub groups_meta: Vec<PlanGroupMeta>,
    pub status: PlanStatus,
    pub created_at: String,
    pub config_version: i64,
}

/// 分组元数据（项内容在 items 里，组只携带展示信息）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanGroupMeta {
    pub physical_target_id: String,
    pub target_path: String,
    pub shared_with_tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    pub kind: TaskKind,
    pub trigger: TaskTrigger,
    pub library_id: Option<String>,
    pub plan_id: Option<String>,
    pub status: TaskStatus,
    pub counts: TaskCounts,
    pub error: Option<AppError>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskItemRow {
    pub id: String,
    pub task_id: String,
    pub mapping_id: Option<String>,
    pub skill_name: String,
    pub target_path: String,
    pub action: PlanAction,
    pub status: TaskItemStatus,
    pub error: Option<AppError>,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub pre_digest: Option<String>,
    pub post_digest: Option<String>,
    pub snapshot_id: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Clone)]
pub struct SnapshotRow {
    pub id: String,
    pub task_item_id: Option<String>,
    pub mapping_id: Option<String>,
    pub target_path: String,
    pub existed_before: bool,
    pub backup_dir: Option<String>,
    pub digest: Option<String>,
    pub bytes: u64,
    pub pinned: bool,
    pub pruned_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct TransactionRow {
    pub id: String,
    pub task_item_id: Option<String>,
    pub mapping_id: Option<String>,
    pub physical_target_id: Option<String>,
    pub skill_dir_name: String,
    pub phase: String,
    pub staging_path: Option<String>,
    pub old_path: Option<String>,
    pub target_path: String,
    pub backup_id: Option<String>,
    pub expected_source_digest: Option<String>,
    pub expected_old_digest: Option<String>,
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn parse_dt_pub(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

impl Store {
    /// 打开（必要时创建）数据库并执行迁移；迁移失败时从备份恢复（§11.2）。
    pub fn open(data_dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(data_dir).map_err(AppError::from)?;
        let db_path = data_dir.join("app.db");
        let backup_path = data_dir.join("app.db.pre-migration.bak");
        let existed = db_path.exists();
        if existed {
            std::fs::copy(&db_path, &backup_path).map_err(AppError::from)?;
        }
        let conn = Connection::open(&db_path).map_err(AppError::from)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(AppError::from)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(AppError::from)?;
        let store = Self {
            conn: Mutex::new(conn),
            db_path: db_path.clone(),
        };
        if let Err(e) = store.migrate() {
            drop(store);
            if existed {
                let _ = std::fs::copy(&backup_path, &db_path);
            }
            return Err(AppError::internal(format!("数据库迁移失败，已从备份恢复：{e}")));
        }
        Self::open_existing(&db_path)
    }

    fn open_existing(db_path: &Path) -> AppResult<Self> {
        let conn = Connection::open(db_path).map_err(AppError::from)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(AppError::from)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(AppError::from)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: db_path.to_path_buf(),
        })
    }

    fn migrate(&self) -> AppResult<()> {
        let current: i64 = self.conn().query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        self.conn().execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations(
                   version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
            )
            .map_err(AppError::from)?;
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let version = (i + 1) as i64;
            if version > current {
                self.conn().execute_batch(sql).map_err(AppError::from)?;
                self.conn().execute(
                        "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                        params![version, now_iso()],
                    )
                    .map_err(AppError::from)?;
            }
        }
        Ok(())
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // ------------------------------------------------------------------
    // 设置
    // ------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self.conn().query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn().execute(
                "INSERT INTO settings(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )
            .map_err(AppError::from)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 技能库
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_library(
        &self,
        id: &str,
        display_name: &str,
        canonical_path: &str,
        source_root: Option<&str>,
        dir_identity: Option<&str>,
        scan_mode: &str,
        skill_filter: Option<&[String]>,
    ) -> AppResult<()> {
        self.conn().execute(
            "INSERT INTO libraries(id, display_name, canonical_path, source_root, dir_identity,
               scan_mode, skill_filter, created_at, last_opened_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
            params![
                id,
                display_name,
                canonical_path,
                source_root,
                dir_identity,
                scan_mode,
                skill_filter.map(serde_json::to_string).transpose()?,
                now_iso()
            ],
        )?;
        Ok(())
    }

    fn row_to_library(r: &rusqlite::Row) -> rusqlite::Result<LibraryRow> {
        let skill_filter: Option<String> = r.get("skill_filter")?;
        let ignore: String = r.get("ignore_patterns")?;
        Ok(LibraryRow {
            id: r.get("id")?,
            display_name: r.get("display_name")?,
            canonical_path: r.get("canonical_path")?,
            source_root: r.get("source_root")?,
            dir_identity: r.get("dir_identity")?,
            scan_mode: r.get("scan_mode")?,
            skill_filter: skill_filter.and_then(|s| serde_json::from_str(&s).ok()),
            ignore_patterns: serde_json::from_str(&ignore).unwrap_or_default(),
            config_version: r.get("config_version")?,
            pinned: r.get::<_, i64>("pinned")? != 0,
            archived: r.get::<_, i64>("archived")? != 0,
            last_opened_at: r.get("last_opened_at")?,
            created_at: r.get("created_at")?,
        })
    }

    const LIB_COLS: &'static str =
        "id, display_name, canonical_path, source_root, dir_identity, scan_mode, skill_filter,
         ignore_patterns, config_version, pinned, archived, last_opened_at, created_at";

    pub fn get_library(&self, id: &str) -> AppResult<LibraryRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM libraries WHERE id=?1", Self::LIB_COLS),
                params![id],
                Self::row_to_library,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("技能库", id))
    }

    pub fn find_library_by_identity(&self, identity: &str) -> AppResult<Option<LibraryRow>> {
        Ok(self.conn().query_row(
                &format!(
                    "SELECT {} FROM libraries WHERE dir_identity=?1 AND archived=0",
                    Self::LIB_COLS
                ),
                params![identity],
                Self::row_to_library,
            )
            .optional()?)
    }

    pub fn list_libraries(&self, include_archived: bool) -> AppResult<Vec<LibraryRow>> {
        let sql = if include_archived {
            format!("SELECT {} FROM libraries ORDER BY pinned DESC, last_opened_at DESC", Self::LIB_COLS)
        } else {
            format!(
                "SELECT {} FROM libraries WHERE archived=0 ORDER BY pinned DESC, last_opened_at DESC",
                Self::LIB_COLS
            )
        };
        let conn_g1 = self.conn();
        let mut stmt = conn_g1.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_library)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn touch_library(&self, id: &str) -> AppResult<()> {
        self.conn().execute(
            "UPDATE libraries SET last_opened_at=?2 WHERE id=?1",
            params![id, now_iso()],
        )?;
        Ok(())
    }

    pub fn set_library_source_root(&self, id: &str, root: &str) -> AppResult<()> {
        let n = self.conn().execute(
            "UPDATE libraries SET source_root=?2 WHERE id=?1",
            params![id, root],
        )?;
        if n == 0 {
            return Err(AppError::not_found("技能库", id));
        }
        Ok(())
    }

    pub fn set_library_pinned(&self, id: &str, pinned: bool) -> AppResult<()> {
        self.conn().execute(
            "UPDATE libraries SET pinned=?2 WHERE id=?1",
            params![id, pinned as i64],
        )?;
        Ok(())
    }

    /// 移除库记录：归档 + 停用映射；历史保留，源文件不动（§7.1）。
    pub fn archive_library(&self, id: &str) -> AppResult<()> {
        self.conn().execute(
            "UPDATE libraries SET archived=1 WHERE id=?1",
            params![id],
        )?;
        self.conn().execute(
            "UPDATE mappings SET enabled=0 WHERE library_id=?1",
            params![id],
        )?;
        Ok(())
    }

    /// 更新忽略规则并递增配置版本（旧配置生成的计划随之失效）。
    pub fn update_library_ignores(&self, id: &str, patterns: &[String]) -> AppResult<i64> {
        self.conn().execute(
            "UPDATE libraries SET ignore_patterns=?2, config_version=config_version+1 WHERE id=?1",
            params![id, serde_json::to_string(patterns)?],
        )?;
        let lib = self.get_library(id)?;
        Ok(lib.config_version)
    }

    // ------------------------------------------------------------------
    // 技能（扫描结果落库）
    // ------------------------------------------------------------------

    /// 用一次扫描结果刷新库内技能： upsert 现有项，未再出现的标记 missing（源已移除）。
    /// 返回 (全部技能行, 本次新移除的 rel_path 列表)。
    pub fn apply_scan(
        &self,
        library_id: &str,
        scanned: &[ScannedSkill],
    ) -> AppResult<(Vec<SkillRow>, Vec<String>)> {
        let now = now_iso();
        let mut newly_missing = Vec::new();
        {
            let conn = self.conn();
            let tx = conn.unchecked_transaction()?;
        let seen: std::collections::HashSet<&str> =
            scanned.iter().map(|s| s.rel_path.as_str()).collect();
        let existing: Vec<(String, String)> = {
            let mut stmt = tx.prepare("SELECT id, rel_path FROM skills WHERE library_id=?1")?;
            let rows = stmt.query_map(params![library_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for (sid, rel) in &existing {
            if !seen.contains(rel.as_str()) {
                let changed = tx.execute(
                    "UPDATE skills SET missing=1, last_scanned_at=?3 WHERE id=?1 AND missing=0",
                    params![sid, rel, now],
                )?;
                if changed > 0 {
                    newly_missing.push(rel.clone());
                }
            }
        }
        for s in scanned {
            let manifest = serde_json::to_string(&s.manifest)?;
            let excluded = serde_json::to_string(&s.excluded_by_ignore)?;
            let issues = serde_json::to_string(&s.issues)?;
            let status = serde_json::to_value(s.status)?
                .as_str()
                .unwrap_or("invalid")
                .to_string();
            let changed = s.last_content_changed_at.map(|t| t.to_rfc3339());
            // 稳定 ID：库内相对路径决定身份（§4.2），重扫不换 ID
            let skill_id = format!("{library_id}:{}", s.rel_path);
            tx.execute(
                "INSERT INTO skills(id, library_id, rel_path, name, description, file_count,
                   total_bytes, digest, manifest_json, excluded_json, validation_status,
                   validation_json, missing, last_content_changed_at, last_scanned_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,?13,?14)
                 ON CONFLICT(library_id, rel_path) DO UPDATE SET
                   name=excluded.name, description=excluded.description,
                   file_count=excluded.file_count, total_bytes=excluded.total_bytes,
                   digest=excluded.digest, manifest_json=excluded.manifest_json,
                   excluded_json=excluded.excluded_json,
                   validation_status=excluded.validation_status,
                   validation_json=excluded.validation_json, missing=0,
                   last_content_changed_at=excluded.last_content_changed_at,
                   last_scanned_at=excluded.last_scanned_at",
                params![
                    skill_id,
                    library_id,
                    s.rel_path,
                    s.name,
                    s.description,
                    s.file_count,
                    s.total_bytes as i64,
                    s.digest,
                    manifest,
                    excluded,
                    status,
                    issues,
                    changed,
                    now
                ],
            )?;
        }
        tx.commit()?;
        } // 释放连接锁后再调用其它方法（std Mutex 不可重入）
        Ok((self.list_skills(library_id)?, newly_missing))
    }

    fn row_to_skill(r: &rusqlite::Row) -> rusqlite::Result<SkillRow> {
        let manifest: String = r.get("manifest_json")?;
        let excluded: String = r.get("excluded_json")?;
        let validation: String = r.get("validation_json")?;
        let status: String = r.get("validation_status")?;
        Ok(SkillRow {
            id: r.get("id")?,
            library_id: r.get("library_id")?,
            rel_path: r.get("rel_path")?,
            name: r.get("name")?,
            description: r.get("description")?,
            file_count: r.get::<_, i64>("file_count")? as u32,
            total_bytes: r.get::<_, i64>("total_bytes")? as u64,
            digest: r.get("digest")?,
            manifest: serde_json::from_str(&manifest).unwrap_or_default(),
            excluded: serde_json::from_str(&excluded).unwrap_or_default(),
            validation_status: serde_json::from_str(&format!("\"{status}\""))
                .unwrap_or(ValidationStatus::Invalid),
            validation: serde_json::from_str(&validation).unwrap_or_default(),
            missing: r.get::<_, i64>("missing")? != 0,
            last_content_changed_at: r.get("last_content_changed_at")?,
        })
    }

    const SKILL_COLS: &'static str =
        "id, library_id, rel_path, name, description, file_count, total_bytes, digest,
         manifest_json, excluded_json, validation_status, validation_json, missing,
         last_content_changed_at";

    pub fn list_skills(&self, library_id: &str) -> AppResult<Vec<SkillRow>> {
        let conn_g2 = self.conn();
        let mut stmt = conn_g2.prepare(&format!(
            "SELECT {} FROM skills WHERE library_id=?1 ORDER BY rel_path",
            Self::SKILL_COLS
        ))?;
        let rows = stmt.query_map(params![library_id], Self::row_to_skill)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_skill(&self, id: &str) -> AppResult<SkillRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM skills WHERE id=?1", Self::SKILL_COLS),
                params![id],
                Self::row_to_skill,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("技能", id))
    }

    // ------------------------------------------------------------------
    // 目标
    // ------------------------------------------------------------------

    pub fn upsert_physical_target(
        &self,
        identity: &str,
        canonical_path: &str,
        availability: Availability,
    ) -> AppResult<PhysicalTargetRow> {
        let avail = serde_json::to_value(availability)?
            .as_str()
            .unwrap_or("exists")
            .to_string();
        self.conn().execute(
            "INSERT INTO physical_targets(id, canonical_path, identity, availability, last_checked_at)
             VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(identity) DO UPDATE SET
               canonical_path=excluded.canonical_path,
               availability=excluded.availability,
               last_checked_at=excluded.last_checked_at",
            params![uuid::Uuid::new_v4().to_string(), canonical_path, identity, avail, now_iso()],
        )?;
        self.conn().query_row(
                "SELECT id, canonical_path, availability FROM physical_targets WHERE identity=?1",
                params![identity],
                |r| {
                    let avail: String = r.get(2)?;
                    Ok(PhysicalTargetRow {
                        id: r.get(0)?,
                        canonical_path: r.get(1)?,
                        availability: serde_json::from_str(&format!("\"{avail}\""))
                            .unwrap_or(Availability::Exists),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| AppError::internal("物理目标写入后读取失败"))
    }

    pub fn get_physical_target(&self, id: &str) -> AppResult<PhysicalTargetRow> {
        self.conn().query_row(
                "SELECT id, canonical_path, availability FROM physical_targets WHERE id=?1",
                params![id],
                |r| {
                    let avail: String = r.get(2)?;
                    Ok(PhysicalTargetRow {
                        id: r.get(0)?,
                        canonical_path: r.get(1)?,
                        availability: serde_json::from_str(&format!("\"{avail}\""))
                            .unwrap_or(Availability::Exists),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("物理目标", id))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_target(
        &self,
        id: &str,
        adapter_id: &str,
        adapter_version: &str,
        scope: TargetScope,
        display_name: &str,
        path_template: &str,
        resolution_source: &str,
        project_root: Option<&str>,
        physical_target_id: &str,
    ) -> AppResult<()> {
        let scope_s = serde_json::to_value(scope)?
            .as_str()
            .unwrap_or("custom")
            .to_string();
        self.conn().execute(
            "INSERT INTO targets(id, adapter_id, adapter_version, scope, display_name,
               path_template, resolution_source, project_root, physical_target_id, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                id, adapter_id, adapter_version, scope_s, display_name, path_template,
                resolution_source, project_root, physical_target_id, now_iso()
            ],
        )?;
        Ok(())
    }

    fn row_to_target(r: &rusqlite::Row) -> rusqlite::Result<TargetRow> {
        let scope: String = r.get("scope")?;
        Ok(TargetRow {
            id: r.get("id")?,
            adapter_id: r.get("adapter_id")?,
            adapter_version: r.get("adapter_version")?,
            scope: serde_json::from_str(&format!("\"{scope}\"")).unwrap_or(TargetScope::Custom),
            display_name: r.get("display_name")?,
            path_template: r.get("path_template")?,
            resolution_source: r.get("resolution_source")?,
            project_root: r.get("project_root")?,
            physical_target_id: r.get("physical_target_id")?,
            enabled: r.get::<_, i64>("enabled")? != 0,
        })
    }

    const TARGET_COLS: &'static str =
        "id, adapter_id, adapter_version, scope, display_name, path_template, resolution_source,
         project_root, physical_target_id, enabled";

    pub fn list_targets(&self) -> AppResult<Vec<TargetRow>> {
        let conn_g3 = self.conn();
        let mut stmt = conn_g3.prepare(&format!(
            "SELECT {} FROM targets WHERE enabled=1 ORDER BY created_at",
            Self::TARGET_COLS
        ))?;
        let rows = stmt.query_map([], Self::row_to_target)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_target(&self, id: &str) -> AppResult<TargetRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM targets WHERE id=?1", Self::TARGET_COLS),
                params![id],
                Self::row_to_target,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("目标", id))
    }

    /// 同一物理目标上的其它启用目标（共享目录的关联工具）。
    pub fn targets_on_physical(&self, physical_target_id: &str) -> AppResult<Vec<TargetRow>> {
        let conn_g4 = self.conn();
        let mut stmt = conn_g4.prepare(&format!(
            "SELECT {} FROM targets WHERE physical_target_id=?1 AND enabled=1",
            Self::TARGET_COLS
        ))?;
        let rows = stmt.query_map(params![physical_target_id], Self::row_to_target)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn disable_target(&self, id: &str) -> AppResult<()> {
        self.conn().execute("UPDATE targets SET enabled=0 WHERE id=?1", params![id])?;
        // 相关映射停用（不动磁盘内容）
        self.conn().execute(
            "UPDATE mappings SET enabled=0 WHERE physical_target_id IN
               (SELECT physical_target_id FROM targets WHERE id=?1)",
            params![id],
        )?;
        Ok(())
    }

    /// 物理目标上的活动映射数（目标卡片「关联技能数」）。
    pub fn count_mappings_on_physical(&self, physical_target_id: &str) -> AppResult<u32> {
        let n: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM mappings WHERE physical_target_id=?1 AND enabled=1",
            params![physical_target_id],
            |r| r.get(0),
        )?;
        Ok(n as u32)
    }

    pub fn update_physical_availability(
        &self,
        id: &str,
        availability: Availability,
    ) -> AppResult<()> {
        let avail = serde_json::to_value(availability)?
            .as_str()
            .unwrap_or("exists")
            .to_string();
        self.conn().execute(
            "UPDATE physical_targets SET availability=?2, last_checked_at=?3 WHERE id=?1",
            params![id, avail, now_iso()],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 映射
    // ------------------------------------------------------------------

    pub fn insert_mapping(
        &self,
        library_id: &str,
        skill_id: &str,
        physical_target_id: &str,
        target_dir_name: &str,
    ) -> AppResult<MappingRow> {
        let id = uuid::Uuid::new_v4().to_string();
        let r = self.conn().execute(
            "INSERT INTO mappings(id, library_id, skill_id, physical_target_id, target_dir_name,
               enabled, owner_library_id, created_at)
             VALUES (?1,?2,?3,?4,?5,1,?6,?7)",
            params![
                id,
                library_id,
                skill_id,
                physical_target_id,
                target_dir_name,
                library_id,
                now_iso()
            ],
        );
        match r {
            Ok(_) => self.get_mapping(&id),
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.extended_code == 2067 => {
                // SQLITE_CONSTRAINT_UNIQUE：同位置已有活动映射 → 复用同库映射或报归属冲突
                if let Some(existing) =
                    self.find_active_mapping(physical_target_id, target_dir_name)?
                {
                    if existing.library_id == library_id && existing.skill_id == skill_id {
                        return Ok(existing);
                    }
                    if existing.library_id == library_id {
                        // 同库内技能改名/移位：停用旧映射后重建
                        self.set_mapping_enabled(&existing.id, false)?;
                        return self.insert_mapping(
                            library_id,
                            skill_id,
                            physical_target_id,
                            target_dir_name,
                        );
                    }
                    return Err(AppError::new(
                        crate::error::ErrorCode::ConflictUnresolved,
                        format!(
                            "目标目录 `{target_dir_name}` 已归属另一个技能库，不能后台抢占；请在预览中明确转移归属"
                        ),
                    )
                    .with_context(serde_json::json!({
                        "targetDirName": target_dir_name,
                        "ownerLibraryId": existing.library_id,
                        "state": "ownership_conflict",
                    })));
                }
                Err(AppError::internal("映射唯一约束冲突但未找到既有映射"))
            }
            Err(e) => Err(AppError::from(e)),
        }
    }

    pub fn get_mapping(&self, id: &str) -> AppResult<MappingRow> {
        self.conn().query_row(
                "SELECT id, library_id, skill_id, physical_target_id, target_dir_name, enabled,
                        owner_library_id, paused_reason FROM mappings WHERE id=?1",
                params![id],
                |r| {
                    Ok(MappingRow {
                        id: r.get(0)?,
                        library_id: r.get(1)?,
                        skill_id: r.get(2)?,
                        physical_target_id: r.get(3)?,
                        target_dir_name: r.get(4)?,
                        enabled: r.get::<_, i64>(5)? != 0,
                        owner_library_id: r.get(6)?,
                        paused_reason: r.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("映射", id))
    }

    pub fn find_active_mapping(
        &self,
        physical_target_id: &str,
        target_dir_name: &str,
    ) -> AppResult<Option<MappingRow>> {
        let conn_g5 = self.conn();
        let mut stmt = conn_g5.prepare(
            "SELECT id, library_id, skill_id, physical_target_id, target_dir_name, enabled,
                    owner_library_id, paused_reason
             FROM mappings WHERE physical_target_id=?1 AND target_dir_name=?2 AND enabled=1",
        )?;
        let rows = stmt.query_map(params![physical_target_id, target_dir_name], |r| {
            Ok(MappingRow {
                id: r.get(0)?,
                library_id: r.get(1)?,
                skill_id: r.get(2)?,
                physical_target_id: r.get(3)?,
                target_dir_name: r.get(4)?,
                enabled: r.get::<_, i64>(5)? != 0,
                owner_library_id: r.get(6)?,
                paused_reason: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?.into_iter().next())
    }

    pub fn mappings_for_skill(&self, skill_id: &str) -> AppResult<Vec<MappingRow>> {
        self.query_mappings("skill_id=?1", &[skill_id])
    }

    pub fn mappings_for_library(&self, library_id: &str) -> AppResult<Vec<MappingRow>> {
        self.query_mappings("library_id=?1", &[library_id])
    }

    fn query_mappings(&self, where_clause: &str, args: &[&str]) -> AppResult<Vec<MappingRow>> {
        let sql = format!(
            "SELECT id, library_id, skill_id, physical_target_id, target_dir_name, enabled,
                    owner_library_id, paused_reason FROM mappings WHERE {where_clause}"
        );
        let conn_g6 = self.conn();
        let mut stmt = conn_g6.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |r| {
            Ok(MappingRow {
                id: r.get(0)?,
                library_id: r.get(1)?,
                skill_id: r.get(2)?,
                physical_target_id: r.get(3)?,
                target_dir_name: r.get(4)?,
                enabled: r.get::<_, i64>(5)? != 0,
                owner_library_id: r.get(6)?,
                paused_reason: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_mapping_enabled(&self, id: &str, enabled: bool) -> AppResult<()> {
        self.conn().execute(
            "UPDATE mappings SET enabled=?2 WHERE id=?1",
            params![id, enabled as i64],
        )?;
        Ok(())
    }

    pub fn set_mapping_paused(&self, id: &str, reason: Option<&str>) -> AppResult<()> {
        self.conn().execute(
            "UPDATE mappings SET paused_reason=?2 WHERE id=?1",
            params![id, reason],
        )?;
        Ok(())
    }

    /// 明确转移归属（§8.2：禁止自动抢占，需用户选择）。
    pub fn transfer_mapping_ownership(&self, id: &str, new_owner_library_id: &str) -> AppResult<()> {
        self.conn().execute(
            "UPDATE mappings SET owner_library_id=?2 WHERE id=?1",
            params![id, new_owner_library_id],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 基线
    // ------------------------------------------------------------------

    pub fn get_baseline(&self, mapping_id: &str) -> AppResult<Option<BaselineRow>> {
        Ok(self.conn().query_row(
                "SELECT mapping_id, digest, manifest_json, task_id, updated_at
                 FROM baselines WHERE mapping_id=?1",
                params![mapping_id],
                |r| {
                    let manifest: String = r.get(2)?;
                    Ok(BaselineRow {
                        mapping_id: r.get(0)?,
                        digest: r.get(1)?,
                        manifest: serde_json::from_str(&manifest).unwrap_or_default(),
                        task_id: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// 写入基线；digest=None 表示「不存在」标记。
    pub fn set_baseline(
        &self,
        mapping_id: &str,
        digest: Option<&str>,
        manifest: &[crate::scanner::FileEntry],
        task_id: Option<&str>,
    ) -> AppResult<()> {
        self.conn().execute(
            "INSERT INTO baselines(mapping_id, digest, manifest_json, task_id, updated_at)
             VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(mapping_id) DO UPDATE SET
               digest=excluded.digest, manifest_json=excluded.manifest_json,
               task_id=excluded.task_id, updated_at=excluded.updated_at",
            params![
                mapping_id,
                digest,
                serde_json::to_string(manifest)?,
                task_id,
                now_iso()
            ],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 计划
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_plan(
        &self,
        id: &str,
        library_id: &str,
        operation: PlanOperation,
        fingerprint: &str,
        items: &[PlanItem],
        groups_meta: &[PlanGroupMeta],
        config_version: i64,
    ) -> AppResult<()> {
        let op = serde_json::to_value(operation)?
            .as_str()
            .unwrap_or("sync")
            .to_string();
        #[derive(serde::Serialize)]
        struct PlanPayload<'a> {
            items: &'a [PlanItem],
            groups_meta: &'a [PlanGroupMeta],
            config_version: i64,
        }
        let payload = serde_json::to_string(&PlanPayload {
            items,
            groups_meta,
            config_version,
        })?;
        self.conn().execute(
            "INSERT INTO sync_plans(id, version, library_id, operation, fingerprint, items_json,
               status, created_at)
             VALUES (?1,1,?2,?3,?4,?5,'active',?6)",
            params![id, library_id, op, fingerprint, payload, now_iso()],
        )?;
        Ok(())
    }

    pub fn get_plan(&self, id: &str) -> AppResult<PlanRow> {
        self.conn().query_row(
                "SELECT id, version, library_id, operation, fingerprint, items_json, status, created_at
                 FROM sync_plans WHERE id=?1",
                params![id],
                |r| {
                    let payload: String = r.get(5)?;
                    let op: String = r.get(3)?;
                    let status: String = r.get(6)?;
                    #[derive(serde::Deserialize)]
                    struct PlanPayload {
                        items: Vec<PlanItem>,
                        groups_meta: Vec<PlanGroupMeta>,
                        config_version: i64,
                    }
                    let parsed: PlanPayload = serde_json::from_str(&payload).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    Ok(PlanRow {
                        id: r.get(0)?,
                        version: r.get(1)?,
                        library_id: r.get(2)?,
                        operation: serde_json::from_str(&format!("\"{op}\""))
                            .unwrap_or(PlanOperation::Sync),
                        fingerprint: r.get(4)?,
                        items: parsed.items,
                        groups_meta: parsed.groups_meta,
                        status: serde_json::from_str(&format!("\"{status}\""))
                            .unwrap_or(PlanStatus::Active),
                        created_at: r.get(7)?,
                        config_version: parsed.config_version,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("同步计划", id))
    }

    pub fn update_plan_items(&self, id: &str, items: &[PlanItem], fingerprint: &str) -> AppResult<i64> {
        let plan = self.get_plan(id)?;
        #[derive(serde::Serialize)]
        struct PlanPayload<'a> {
            items: &'a [PlanItem],
            groups_meta: &'a [PlanGroupMeta],
            config_version: i64,
        }
        let payload = serde_json::to_string(&PlanPayload {
            items,
            groups_meta: &plan.groups_meta,
            config_version: plan.config_version,
        })?;
        self.conn().execute(
            "UPDATE sync_plans SET items_json=?2, fingerprint=?3, version=version+1 WHERE id=?1",
            params![id, payload, fingerprint],
        )?;
        Ok(plan.version + 1)
    }

    pub fn set_plan_status(&self, id: &str, status: PlanStatus) -> AppResult<()> {
        let s = serde_json::to_value(status)?
            .as_str()
            .unwrap_or("active")
            .to_string();
        self.conn().execute("UPDATE sync_plans SET status=?2 WHERE id=?1", params![id, s])?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 任务
    // ------------------------------------------------------------------

    pub fn insert_task(
        &self,
        id: &str,
        kind: TaskKind,
        trigger: TaskTrigger,
        library_id: Option<&str>,
        plan_id: Option<&str>,
    ) -> AppResult<()> {
        let kind = serde_json::to_value(kind)?.as_str().unwrap_or("sync").to_string();
        let trigger = serde_json::to_value(trigger)?
            .as_str()
            .unwrap_or("manual")
            .to_string();
        self.conn().execute(
            "INSERT INTO tasks(id, kind, trigger, library_id, plan_id, status, counts_json, created_at)
             VALUES (?1,?2,?3,?4,?5,'queued','{}',?6)",
            params![id, kind, trigger, library_id, plan_id, now_iso()],
        )?;
        Ok(())
    }

    fn row_to_task(r: &rusqlite::Row) -> rusqlite::Result<TaskRow> {
        let kind: String = r.get("kind")?;
        let trigger: String = r.get("trigger")?;
        let status: String = r.get("status")?;
        let counts: String = r.get("counts_json")?;
        let error: Option<String> = r.get("error_json")?;
        Ok(TaskRow {
            id: r.get("id")?,
            kind: serde_json::from_str(&format!("\"{kind}\"")).unwrap_or(TaskKind::Sync),
            trigger: serde_json::from_str(&format!("\"{trigger}\"")).unwrap_or(TaskTrigger::Manual),
            library_id: r.get("library_id")?,
            plan_id: r.get("plan_id")?,
            status: TaskStatus::parse(&status).unwrap_or(TaskStatus::Failed),
            counts: serde_json::from_str(&counts).unwrap_or_default(),
            error: error.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.get("created_at")?,
            started_at: r.get("started_at")?,
            finished_at: r.get("finished_at")?,
        })
    }

    const TASK_COLS: &'static str =
        "id, kind, trigger, library_id, plan_id, status, counts_json, error_json,
         created_at, started_at, finished_at";

    pub fn get_task(&self, id: &str) -> AppResult<TaskRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM tasks WHERE id=?1", Self::TASK_COLS),
                params![id],
                Self::row_to_task,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("任务", id))
    }

    pub fn list_tasks(&self, active_only: bool) -> AppResult<Vec<TaskRow>> {
        let sql = if active_only {
            format!(
                "SELECT {} FROM tasks WHERE status IN ('queued','running') ORDER BY created_at DESC",
                Self::TASK_COLS
            )
        } else {
            format!("SELECT {} FROM tasks ORDER BY created_at DESC LIMIT 500", Self::TASK_COLS)
        };
        let conn_g7 = self.conn();
        let mut stmt = conn_g7.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_task)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_history(
        &self,
        cursor: Option<&str>,
        limit: u32,
        library_id: Option<&str>,
        status: Option<TaskStatus>,
    ) -> AppResult<Vec<TaskRow>> {
        let mut sql = format!(
            "SELECT {} FROM tasks WHERE kind != 'scan' AND status NOT IN ('queued','running')",
            Self::TASK_COLS
        );
        if library_id.is_some() {
            sql.push_str(" AND library_id = :lib");
        }
        if status.is_some() {
            sql.push_str(" AND status = :status");
        }
        if let Some(c) = cursor {
            let _ = c;
            sql.push_str(" AND created_at < :cursor");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT :limit");
        let conn_g8 = self.conn();
        let mut stmt = conn_g8.prepare(&sql)?;
        let mut named: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        named.push((":limit", &limit));
        let lib_s;
        if let Some(l) = library_id {
            lib_s = l.to_string();
            named.push((":lib", &lib_s));
        }
        let status_s;
        if let Some(s) = status {
            status_s = s.as_str().to_string();
            named.push((":status", &status_s));
        }
        let cursor_s;
        if let Some(c) = cursor {
            cursor_s = c.to_string();
            named.push((":cursor", &cursor_s));
        }
        let rows = stmt.query_map(named.as_slice(), Self::row_to_task)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn update_task_status(&self, id: &str, status: TaskStatus) -> AppResult<()> {
        self.conn().execute(
            "UPDATE tasks SET status=?2,
               started_at = CASE WHEN ?2 = 'running' AND started_at IS NULL THEN ?3 ELSE started_at END,
               finished_at = CASE WHEN ?2 IN ('completed','partial','failed','cancelled','recovery_pending')
                                  THEN ?3 ELSE finished_at END
             WHERE id=?1",
            params![id, status.as_str(), now_iso()],
        )?;
        Ok(())
    }

    pub fn set_task_counts(&self, id: &str, counts: &TaskCounts) -> AppResult<()> {
        self.conn().execute(
            "UPDATE tasks SET counts_json=?2 WHERE id=?1",
            params![id, serde_json::to_string(counts)?],
        )?;
        Ok(())
    }

    pub fn set_task_error(&self, id: &str, error: Option<&AppError>) -> AppResult<()> {
        self.conn().execute(
            "UPDATE tasks SET error_json=?2 WHERE id=?1",
            params![id, error.map(serde_json::to_string).transpose()?],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 任务项
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_task_item(
        &self,
        id: &str,
        task_id: &str,
        mapping_id: Option<&str>,
        skill_name: &str,
        target_path: &str,
        action: PlanAction,
        bytes_total: u64,
        seq: i64,
    ) -> AppResult<()> {
        let action = serde_json::to_value(action)?
            .as_str()
            .unwrap_or("skip")
            .to_string();
        self.conn().execute(
            "INSERT INTO task_items(id, task_id, mapping_id, skill_name, target_path, action,
               status, bytes_total, seq)
             VALUES (?1,?2,?3,?4,?5,?6,'pending',?7,?8)",
            params![id, task_id, mapping_id, skill_name, target_path, action, bytes_total as i64, seq],
        )?;
        Ok(())
    }

    fn row_to_task_item(r: &rusqlite::Row) -> rusqlite::Result<TaskItemRow> {
        let action: String = r.get("action")?;
        let status: String = r.get("status")?;
        let error: Option<String> = r.get("error_json")?;
        Ok(TaskItemRow {
            id: r.get("id")?,
            task_id: r.get("task_id")?,
            mapping_id: r.get("mapping_id")?,
            skill_name: r.get("skill_name")?,
            target_path: r.get("target_path")?,
            action: serde_json::from_str(&format!("\"{action}\"")).unwrap_or(PlanAction::Skip),
            status: TaskItemStatus::parse(&status),
            error: error.and_then(|s| serde_json::from_str(&s).ok()),
            bytes_processed: r.get::<_, i64>("bytes_processed")? as u64,
            bytes_total: r.get::<_, i64>("bytes_total")? as u64,
            pre_digest: r.get("pre_digest")?,
            post_digest: r.get("post_digest")?,
            snapshot_id: None,
            seq: r.get("seq")?,
        })
    }

    const TASK_ITEM_COLS: &'static str =
        "id, task_id, mapping_id, skill_name, target_path, action, status, error_json,
         bytes_processed, bytes_total, pre_digest, post_digest, seq";

    pub fn list_task_items(&self, task_id: &str) -> AppResult<Vec<TaskItemRow>> {
        let mut items: Vec<TaskItemRow> = {
            let conn = self.conn();
            let mut stmt = conn.prepare(&format!(
                "SELECT {} FROM task_items WHERE task_id=?1 ORDER BY seq",
                Self::TASK_ITEM_COLS
            ))?;
            let rows = stmt.query_map(params![task_id], Self::row_to_task_item)?;
            rows.collect::<Result<Vec<_>, _>>()?
        }; // 释放锁后再逐项查快照（std Mutex 不可重入）
        for item in &mut items {
            item.snapshot_id = self.conn().query_row(
                    "SELECT id FROM snapshots WHERE task_item_id=?1 ORDER BY created_at DESC LIMIT 1",
                    params![item.id],
                    |r| r.get(0),
                )
                .optional()?;
        }
        Ok(items)
    }

    pub fn get_task_item(&self, id: &str) -> AppResult<TaskItemRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM task_items WHERE id=?1", Self::TASK_ITEM_COLS),
                params![id],
                Self::row_to_task_item,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("任务项", id))
    }

    pub fn update_task_item_status(
        &self,
        id: &str,
        status: TaskItemStatus,
        error: Option<&AppError>,
    ) -> AppResult<()> {
        self.conn().execute(
            "UPDATE task_items SET status=?2, error_json=?3 WHERE id=?1",
            params![
                id,
                status.as_str(),
                error.map(serde_json::to_string).transpose()?
            ],
        )?;
        Ok(())
    }

    pub fn update_task_item_digests(
        &self,
        id: &str,
        pre: Option<&str>,
        post: Option<&str>,
        bytes: u64,
    ) -> AppResult<()> {
        self.conn().execute(
            "UPDATE task_items SET pre_digest=?2, post_digest=?3, bytes_processed=?4 WHERE id=?1",
            params![id, pre, post, bytes as i64],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 快照
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_snapshot(
        &self,
        id: &str,
        task_item_id: Option<&str>,
        mapping_id: Option<&str>,
        target_path: &str,
        existed_before: bool,
        backup_dir: Option<&str>,
        digest: Option<&str>,
        bytes: u64,
    ) -> AppResult<()> {
        self.conn().execute(
            "INSERT INTO snapshots(id, task_item_id, mapping_id, target_path, existed_before,
               backup_dir, digest, bytes, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id,
                task_item_id,
                mapping_id,
                target_path,
                existed_before as i64,
                backup_dir,
                digest,
                bytes as i64,
                now_iso()
            ],
        )?;
        Ok(())
    }

    fn row_to_snapshot(r: &rusqlite::Row) -> rusqlite::Result<SnapshotRow> {
        Ok(SnapshotRow {
            id: r.get("id")?,
            task_item_id: r.get("task_item_id")?,
            mapping_id: r.get("mapping_id")?,
            target_path: r.get("target_path")?,
            existed_before: r.get::<_, i64>("existed_before")? != 0,
            backup_dir: r.get("backup_dir")?,
            digest: r.get("digest")?,
            bytes: r.get::<_, i64>("bytes")? as u64,
            pinned: r.get::<_, i64>("pinned")? != 0,
            pruned_at: r.get("pruned_at")?,
            created_at: r.get("created_at")?,
        })
    }

    const SNAPSHOT_COLS: &'static str =
        "id, task_item_id, mapping_id, target_path, existed_before, backup_dir, digest, bytes,
         pinned, pruned_at, created_at";

    pub fn get_snapshot(&self, id: &str) -> AppResult<SnapshotRow> {
        self.conn().query_row(
                &format!("SELECT {} FROM snapshots WHERE id=?1", Self::SNAPSHOT_COLS),
                params![id],
                Self::row_to_snapshot,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("快照", id))
    }

    pub fn list_snapshots(&self, mapping_id: Option<&str>) -> AppResult<Vec<SnapshotRow>> {
        let (sql, params_vec): (String, Vec<String>) = match mapping_id {
            Some(m) => (
                format!("SELECT {} FROM snapshots WHERE mapping_id=?1 ORDER BY created_at DESC", Self::SNAPSHOT_COLS),
                vec![m.to_string()],
            ),
            None => (
                format!("SELECT {} FROM snapshots ORDER BY created_at DESC", Self::SNAPSHOT_COLS),
                vec![],
            ),
        };
        let conn_g10 = self.conn();
        let mut stmt = conn_g10.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), Self::row_to_snapshot)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn mark_snapshot_pruned(&self, id: &str) -> AppResult<()> {
        self.conn().execute(
            "UPDATE snapshots SET pruned_at=?2 WHERE id=?1",
            params![id, now_iso()],
        )?;
        Ok(())
    }

    /// 每个映射最近一次可用快照（受保护，不自动清理；§7.4）
    /// 每个映射最近一次可用快照（受保护，不自动清理；§7.4）。
    /// 利用 SQLite 特性：单一 MAX 聚合时裸列取自最大值所在行。
    pub fn latest_snapshot_per_mapping(&self) -> AppResult<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, MAX(created_at) FROM snapshots
             WHERE mapping_id IS NOT NULL AND pruned_at IS NULL
             GROUP BY mapping_id",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ------------------------------------------------------------------
    // 恢复事务
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_transaction(&self, tx: &TransactionRow) -> AppResult<()> {
        self.conn().execute(
            "INSERT INTO recovery_transactions(id, task_item_id, mapping_id, physical_target_id,
               skill_dir_name, phase, staging_path, old_path, target_path, backup_id,
               expected_source_digest, expected_old_digest, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
             ON CONFLICT(id) DO UPDATE SET
               phase=excluded.phase, staging_path=excluded.staging_path,
               old_path=excluded.old_path, backup_id=excluded.backup_id,
               expected_source_digest=excluded.expected_source_digest,
               expected_old_digest=excluded.expected_old_digest,
               updated_at=excluded.updated_at",
            params![
                tx.id,
                tx.task_item_id,
                tx.mapping_id,
                tx.physical_target_id,
                tx.skill_dir_name,
                tx.phase,
                tx.staging_path,
                tx.old_path,
                tx.target_path,
                tx.backup_id,
                tx.expected_source_digest,
                tx.expected_old_digest,
                now_iso()
            ],
        )?;
        Ok(())
    }

    pub fn get_transaction(&self, id: &str) -> AppResult<Option<TransactionRow>> {
        Ok(self.conn().query_row(
                "SELECT id, task_item_id, mapping_id, physical_target_id, skill_dir_name, phase,
                        staging_path, old_path, target_path, backup_id, expected_source_digest,
                        expected_old_digest
                 FROM recovery_transactions WHERE id=?1",
                params![id],
                |r| {
                    Ok(TransactionRow {
                        id: r.get(0)?,
                        task_item_id: r.get(1)?,
                        mapping_id: r.get(2)?,
                        physical_target_id: r.get(3)?,
                        skill_dir_name: r.get(4)?,
                        phase: r.get(5)?,
                        staging_path: r.get(6)?,
                        old_path: r.get(7)?,
                        target_path: r.get(8)?,
                        backup_id: r.get(9)?,
                        expected_source_digest: r.get(10)?,
                        expected_old_digest: r.get(11)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn open_transactions(&self) -> AppResult<Vec<TransactionRow>> {
        let conn_g12 = self.conn();
        let mut stmt = conn_g12.prepare(
            "SELECT id, task_item_id, mapping_id, physical_target_id, skill_dir_name, phase,
                    staging_path, old_path, target_path, backup_id, expected_source_digest,
                    expected_old_digest
             FROM recovery_transactions WHERE phase NOT IN ('done','aborted','manual')",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TransactionRow {
                id: r.get(0)?,
                task_item_id: r.get(1)?,
                mapping_id: r.get(2)?,
                physical_target_id: r.get(3)?,
                skill_dir_name: r.get(4)?,
                phase: r.get(5)?,
                staging_path: r.get(6)?,
                old_path: r.get(7)?,
                target_path: r.get(8)?,
                backup_id: r.get(9)?,
                expected_source_digest: r.get(10)?,
                expected_old_digest: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_transaction_phase(&self, id: &str, phase: &str) -> AppResult<()> {
        self.conn().execute(
            "UPDATE recovery_transactions SET phase=?2, updated_at=?3 WHERE id=?1",
            params![id, phase, now_iso()],
        )?;
        Ok(())
    }

    /// 事务目录下是否存在无事务记录的未知子目录（§8.4.1：保留名称存在未知内容则阻止）。
    pub fn known_transaction_dirs(&self) -> AppResult<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, staging_path, old_path FROM recovery_transactions")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows.collect::<Result<Vec<_>, _>>()? {
            out.push(row.0);
            out.extend(row.1);
            out.extend(row.2);
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // 组装视图
    // ------------------------------------------------------------------

    pub fn task_snapshot_view(&self, task_id: &str) -> AppResult<TaskSnapshotView> {
        let t = self.get_task(task_id)?;
        let items = self.list_task_items(task_id)?;
        Ok(TaskSnapshotView {
            task_id: t.id,
            kind: t.kind,
            trigger: t.trigger,
            status: t.status,
            created_at: t.created_at,
            started_at: t.started_at,
            finished_at: t.finished_at,
            counts: t.counts,
            items: items
                .into_iter()
                .map(|i| TaskItemView {
                    item_id: i.id,
                    mapping_id: i.mapping_id,
                    skill_name: i.skill_name,
                    target_path: i.target_path,
                    action: i.action,
                    status: i.status,
                    error: i.error,
                    bytes_processed: i.bytes_processed,
                    bytes_total: i.bytes_total,
                    snapshot_id: i.snapshot_id,
                })
                .collect(),
            error: t.error,
        })
    }

    pub fn plan_view(&self, plan: &PlanRow) -> AppResult<SyncPlan> {
        let mut groups: Vec<PlanGroup> = Vec::new();
        for meta in &plan.groups_meta {
            let items: Vec<PlanItem> = plan
                .items
                .iter()
                .filter(|i| {
                    self.item_in_group(i, meta).unwrap_or(false)
                })
                .cloned()
                .collect();
            groups.push(PlanGroup {
                physical_target_id: meta.physical_target_id.clone(),
                target_path: meta.target_path.clone(),
                shared_with_tools: meta.shared_with_tools.clone(),
                items,
            });
        }
        let mut summary = PlanSummary::default();
        for item in &plan.items {
            match item.action {
                PlanAction::Create => summary.create_count += 1,
                PlanAction::Update | PlanAction::Adopt | PlanAction::Reinstall => {
                    summary.update_count += 1
                }
                PlanAction::Remove => summary.remove_count += 1,
                PlanAction::Restore => summary.restore_count += 1,
                PlanAction::Blocked => summary.blocked_count += 1,
                PlanAction::Skip => summary.skip_count += 1,
                PlanAction::Overwrite | PlanAction::TakeOver => summary.update_count += 1,
            }
            if item.conflict.is_some() && item.decision.is_none() {
                summary.conflict_count += 1;
            }
            if item.selected && item.action != PlanAction::Skip && item.action != PlanAction::Blocked
            {
                summary.executable_count += 1;
                summary.total_bytes += item
                    .file_changes
                    .iter()
                    .filter(|f| f.kind != "delete")
                    .map(|f| f.bytes)
                    .sum::<u64>();
            }
        }
        Ok(SyncPlan {
            plan_id: plan.id.clone(),
            plan_version: plan.version,
            library_id: plan.library_id.clone(),
            operation: plan.operation,
            status: plan.status,
            created_at: plan.created_at.clone(),
            config_version: plan.config_version,
            groups,
            summary,
        })
    }

    fn item_in_group(&self, item: &PlanItem, meta: &PlanGroupMeta) -> AppResult<bool> {
        let mapping = self.get_mapping(&item.mapping_id)?;
        Ok(mapping.physical_target_id == meta.physical_target_id)
    }
}

/// 供命令层判断「任务终态」：§11.1 全部完成/部分完成/失败/已取消/待恢复。
pub fn final_task_status(counts: &TaskCounts, cancelled: bool, recovery_pending: bool) -> TaskStatus {
    if recovery_pending {
        return TaskStatus::RecoveryPending;
    }
    if cancelled {
        return TaskStatus::Cancelled;
    }
    if counts.failed == 0 {
        TaskStatus::Completed
    } else if counts.success > 0 || counts.skipped > 0 {
        TaskStatus::Partial
    } else {
        TaskStatus::Failed
    }
}
