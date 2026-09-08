//! 备份与快照（需求 §7.4）。
//!
//! - 成功更新前与整项移除前必须备份；备份失败则该项不执行
//! - 首次创建记录「原先不存在」（existed_before = 0），恢复时可撤销未漂移的首次创建
//! - 保留策略：默认 30 天 / 2 GiB 软上限；每映射至少保留最近一次可用备份；
//!   未完成事务引用与用户固定的快照不自动删除；历史保留，恢复入口标记「备份已过期」

use crate::contract::BackupStats;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::fsops;
use crate::scanner;
use crate::storage::{SnapshotRow, Store};
use std::path::{Path, PathBuf};

pub const DEFAULT_RETENTION_DAYS: u32 = 30;
pub const DEFAULT_SOFT_CAP_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB

#[derive(Clone)]
pub struct BackupStore {
    root: PathBuf, // <data_dir>/backups
}

impl BackupStore {
    pub fn new(data_dir: &Path) -> AppResult<Self> {
        let root = data_dir.join("backups");
        std::fs::create_dir_all(&root).map_err(AppError::from)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn snapshot_dir(&self, backup_dir: &str) -> PathBuf {
        self.root.join(backup_dir)
    }

    /// 创建快照。目标存在则复制并校验摘要；不存在则登记「原先不存在」记录。
    /// 任何一步失败返回 backup_failed，调用方（执行器）据此阻止该项覆盖（AC-19）。
    pub fn create_snapshot(
        &self,
        store: &Store,
        task_item_id: Option<&str>,
        mapping_id: Option<&str>,
        target_path: &Path,
    ) -> AppResult<SnapshotRow> {
        let id = uuid::Uuid::new_v4().to_string();
        // 用 symlink_metadata 判定存在性：悬空符号链接 exists() 会误判为不存在（§8.5 不跟随）
        let meta = match std::fs::symlink_metadata(target_path) {
            Ok(m) => Some(m),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(AppError::from(e)),
        };
        if let Some(m) = &meta {
            if crate::windows_paths::is_reparse_point(m) {
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    format!(
                        "目标是符号链接/目录联接，不备份、不改动；请手动处理：{}",
                        target_path.display()
                    ),
                ));
            }
        }
        let exists = meta.is_some();
        if !exists {
            store
                .insert_snapshot(&id, task_item_id, mapping_id, &target_path.to_string_lossy(), false, None, None, 0)
                .map_err(|e| AppError::new(ErrorCode::BackupFailed, format!("备份登记失败：{e}")))?;
            return store.get_snapshot(&id);
        }
        let (expected_digest, manifest) = scanner::digest_directory(target_path)?
            .ok_or_else(|| AppError::new(ErrorCode::BackupFailed, "备份前读取目标失败"))?;
        // 备份卷空间预检（§8.5：备份空间与目标卷空间分别估算）
        let target_bytes: u64 = manifest.iter().map(|f| f.size).sum();
        if let Some(free) = crate::windows_paths::free_space(&self.root) {
            if free < target_bytes {
                return Err(AppError::new(
                    ErrorCode::DiskFull,
                    format!("备份空间不足：需要 {} 字节，可用 {free} 字节", target_bytes),
                ));
            }
        }
        let dir_name = id.clone();
        let dst = self.root.join(&dir_name);
        let (_, bytes) = fsops::copy_tree(target_path, &dst).map_err(|e| {
            AppError::new(ErrorCode::BackupFailed, format!("备份复制失败：{e}"))
                .with_context(serde_json::json!({ "targetPath": target_path.to_string_lossy() }))
        })?;
        // 备份完整性校验：复制后重新计算摘要（§7.4 可验证备份）
        let actual = scanner::digest_directory(&dst)?
            .map(|(d, _)| d)
            .unwrap_or_default();
        if actual != expected_digest {
            let _ = std::fs::remove_dir_all(&dst);
            return Err(AppError::new(
                ErrorCode::BackupFailed,
                "备份校验失败：快照摘要与目标不一致，该项不会执行覆盖",
            ));
        }
        store
            .insert_snapshot(
                &id,
                task_item_id,
                mapping_id,
                &target_path.to_string_lossy(),
                true,
                Some(&dir_name),
                Some(&expected_digest),
                bytes,
            )
            .map_err(|e| AppError::new(ErrorCode::BackupFailed, format!("备份登记失败：{e}")))?;
        store.get_snapshot(&id)
    }

    /// 恢复前校验快照完整性（§7.4：缺失或损坏时阻止恢复）。
    pub fn verify(&self, snapshot: &SnapshotRow) -> AppResult<bool> {
        if snapshot.pruned_at.is_some() {
            return Ok(false);
        }
        if !snapshot.existed_before {
            // 「原先不存在」记录无内容可校验
            return Ok(true);
        }
        let Some(dir) = &snapshot.backup_dir else {
            return Ok(false);
        };
        let path = self.root.join(dir);
        let Some(expected) = &snapshot.digest else {
            return Ok(false);
        };
        match scanner::digest_directory(&path)? {
            Some((actual, _)) => Ok(&actual == expected),
            None => Ok(false),
        }
    }

    pub fn stats(&self, store: &Store) -> AppResult<BackupStats> {
        let retention = store
            .get_setting("backup_retention_days")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_RETENTION_DAYS);
        let soft_cap = store
            .get_setting("backup_soft_cap_bytes")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_SOFT_CAP_BYTES);
        let snapshots = store.list_snapshots(None)?;
        let protected_ids = store.latest_snapshot_per_mapping()?;
        let open_txs = store.open_transactions()?;
        let tx_backups: Vec<String> = open_txs.iter().filter_map(|t| t.backup_id.clone()).collect();
        let mut total = 0u64;
        let mut reclaimable = 0u64;
        let mut protected = 0u64;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention as i64);
        for s in &snapshots {
            if s.pruned_at.is_some() {
                continue;
            }
            total += s.bytes;
            let is_protected = s.pinned
                || protected_ids.contains(&s.id)
                || tx_backups.contains(&s.id);
            let expired = crate::storage::parse_dt_pub(&s.created_at) < cutoff;
            if is_protected {
                protected += s.bytes;
            } else if expired {
                reclaimable += s.bytes;
            }
        }
        Ok(BackupStats {
            total_bytes: total,
            reclaimable_bytes: reclaimable,
            protected_bytes: protected,
            retention_days: retention,
            soft_cap_bytes: soft_cap,
        })
    }

    /// 保留清理：先清超过保留期且未保护的，再按时间淘汰直到低于软上限（§7.4）。
    pub fn sweep(&self, store: &Store) -> AppResult<u32> {
        let stats = self.stats(store)?;
        let protected_ids = store.latest_snapshot_per_mapping()?;
        let open_txs = store.open_transactions()?;
        let tx_backups: Vec<String> = open_txs.iter().filter_map(|t| t.backup_id.clone()).collect();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(stats.retention_days as i64);
        let mut snapshots = store.list_snapshots(None)?;
        snapshots.retain(|s| s.pruned_at.is_none());
        let is_protected = |s: &SnapshotRow| {
            s.pinned || protected_ids.contains(&s.id) || tx_backups.contains(&s.id)
        };
        let mut pruned = 0u32;
        // 第一遍：过期且未保护
        for s in &snapshots {
            if !is_protected(s) && crate::storage::parse_dt_pub(&s.created_at) < cutoff {
                self.prune(store, s)?;
                pruned += 1;
            }
        }
        // 第二遍：仍超软上限时按时间淘汰未保护的
        let mut total = stats.total_bytes;
        if total > stats.soft_cap_bytes {
            let mut rest = snapshots;
            rest.retain(|s| !is_protected(s) && s.pruned_at.is_none());
            rest.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            for s in rest {
                if total <= stats.soft_cap_bytes {
                    break;
                }
                total = total.saturating_sub(s.bytes);
                self.prune(store, &s)?;
                pruned += 1;
            }
        }
        Ok(pruned)
    }

    fn prune(&self, store: &Store, s: &SnapshotRow) -> AppResult<()> {
        if let Some(dir) = &s.backup_dir {
            let path = self.root.join(dir);
            if path.exists() {
                std::fs::remove_dir_all(&path).map_err(AppError::from)?;
            }
        }
        store.mark_snapshot_pruned(&s.id)?;
        Ok(())
    }
}
