//! 中断恢复与历史恢复（需求 §6.5、§7.4、§8.4.5/8.4.6）。
//!
//! 启动恢复：依据磁盘实际目录和摘要判断「继续登记成功」或「恢复旧版本」；
//! 发现未知第三方内容则停止并保留现场（不覆盖未知内容），发出 recovery.required。
//! 历史恢复：生成普通恢复计划，经执行器事务流程执行，恢复前再快照当前目标。

use crate::backup::BackupStore;
use crate::contract::*;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::fsops;
use crate::scanner;
use crate::storage::{PlanGroupMeta, Store};
use std::path::{Path, PathBuf};

pub struct Recovery<'a> {
    pub store: &'a Store,
    pub backups: &'a BackupStore,
    pub data_dir: &'a Path,
}

pub struct RecoveryOutcome {
    pub auto_recovered: Vec<String>,
    pub manual: Vec<RecoveryRequiredEvent>,
}

impl<'a> Recovery<'a> {
    /// 应用启动时扫描未完成事务（§8.4.6）。
    pub fn startup_recovery(&self, seq: &mut dyn FnMut() -> u64) -> AppResult<RecoveryOutcome> {
        let mut out = RecoveryOutcome {
            auto_recovered: Vec::new(),
            manual: Vec::new(),
        };
        let open = self.store.open_transactions()?;
        for tx in open {
            match self.recover_one(&tx) {
                Ok(recovered) => {
                    out.auto_recovered.push(format!(
                        "{}：{}",
                        tx.target_path,
                        if recovered { "已恢复/确认" } else { "无需处理" }
                    ));
                }
                Err(reason) => {
                    let _ = self.store.set_transaction_phase(&tx.id, "manual");
                    if let Some(mapping_id) = &tx.mapping_id {
                        let _ = self.store.set_mapping_paused(mapping_id, Some("recovery_pending"));
                    }
                    out.manual.push(RecoveryRequiredEvent {
                        task_id: tx.task_item_id.clone(),
                        seq: seq(),
                        transaction_id: tx.id.clone(),
                        target_path: tx.target_path.clone(),
                        skill_name: Some(tx.skill_dir_name.clone()),
                        reason: reason.message,
                        recoverable: "manual".to_string(),
                    });
                }
            }
        }
        // 中断时仍处于 queued/running 的任务：按项现状收尾（§11.1 终态）
        for task in self.store.list_tasks(true)? {
            let items = self.store.list_task_items(&task.id)?;
            for item in &items {
                if matches!(item.status, TaskItemStatus::Pending | TaskItemStatus::Running) {
                    // 有关联事务的项由事务恢复决定终态
                    let has_tx = self
                        .store
                        .open_transactions()?
                        .iter()
                        .any(|t| t.task_item_id.as_deref() == Some(item.id.as_str()));
                    if !has_tx {
                        let e = AppError::new(
                            ErrorCode::ValidationFailed,
                            "应用在执行期间退出，该项未完成",
                        )
                        .retryable(false);
                        let _ = self.store.update_task_item_status(
                            &item.id,
                            TaskItemStatus::Cancelled,
                            Some(&e),
                        );
                    }
                }
            }
            let items = self.store.list_task_items(&task.id)?;
            let mut counts = TaskCounts::default();
            for i in &items {
                counts.total += 1;
                match i.status {
                    TaskItemStatus::Success => counts.success += 1,
                    TaskItemStatus::Failed => counts.failed += 1,
                    TaskItemStatus::Skipped => counts.skipped += 1,
                    TaskItemStatus::ConflictPending => counts.conflict_pending += 1,
                    TaskItemStatus::Cancelled => counts.cancelled += 1,
                    _ => {}
                }
            }
            let manual_pending = items
                .iter()
                .any(|i| i.status == TaskItemStatus::RecoveryPending);
            let cancelled = items.iter().any(|i| i.status == TaskItemStatus::Cancelled);
            let status =
                crate::storage::final_task_status(&counts, cancelled, manual_pending);
            let _ = self.store.set_task_counts(&task.id, &counts);
            let _ = self.store.update_task_status(&task.id, status);
        }
        Ok(out)
    }

    /// 依据磁盘实际状态恢复单个事务（§8.4.5/8.4.6）。
    /// 决策以「磁盘实际目录 + 摘要」为准，日志阶段用于区分移除类事务的语义；
    /// 任何未知内容都不覆盖，转人工。
    fn recover_one(&self, tx: &crate::storage::TransactionRow) -> AppResult<bool> {
        let target = PathBuf::from(&tx.target_path);
        let old = tx.old_path.as_ref().map(PathBuf::from);
        let tx_root = old
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| {
                tx.staging_path
                    .as_ref()
                    .and_then(|p| PathBuf::from(p).parent().map(|x| x.to_path_buf()))
            });
        let target_digest = scanner::digest_directory(&target)?.map(|(d, _)| d);
        let expected_src = tx.expected_source_digest.as_deref();
        let expected_old = tx.expected_old_digest.as_deref();
        let old_digest = match &old {
            Some(p) if p.exists() => scanner::digest_directory(p)?.map(|(d, _)| d),
            _ => None,
        };
        // 移除/恢复为不存在：期望终态是目标缺席
        let remove_like = expected_src.is_none() && expected_old.is_some();

        let cleanup = |tx_root: &Option<PathBuf>| -> AppResult<()> {
            if let Some(root) = tx_root {
                if root.exists() {
                    std::fs::remove_dir_all(root).map_err(AppError::from)?;
                }
            }
            let journal = self.data_dir.join("journals").join(format!("{}.json", tx.id));
            let _ = std::fs::remove_file(journal);
            Ok(())
        };
        let abort_item = |msg: &str| {
            if let Some(item_id) = &tx.task_item_id {
                let e = AppError::new(ErrorCode::ValidationFailed, msg).retryable(false);
                let _ = self
                    .store
                    .update_task_item_status(item_id, TaskItemStatus::Cancelled, Some(&e));
            }
        };

        // 1) 新内容已完整就位 → 继续登记成功
        if let (Some(t), Some(e)) = (target_digest.as_deref(), expected_src) {
            if t == e {
                let (_, manifest) = scanner::digest_directory(&target)?.unwrap();
                if let Some(mapping_id) = &tx.mapping_id {
                    self.store.set_baseline(
                        mapping_id,
                        expected_src,
                        &manifest,
                        tx.task_item_id.as_deref(),
                    )?;
                }
                if let Some(item_id) = &tx.task_item_id {
                    let _ = self
                        .store
                        .update_task_item_digests(item_id, expected_old, expected_src, 0);
                    let _ = self
                        .store
                        .update_task_item_status(item_id, TaskItemStatus::Success, None);
                }
                cleanup(&tx_root)?;
                self.store.set_transaction_phase(&tx.id, "done")?;
                return Ok(true);
            }
        }
        // 2) 旧内容仍在原位 → 事务未造成变更，作废并清理暂存
        if let (Some(t), Some(e)) = (target_digest.as_deref(), expected_old) {
            if t == e {
                abort_item("同步在提交前中断，目标内容未变更");
                cleanup(&tx_root)?;
                self.store.set_transaction_phase(&tx.id, "aborted")?;
                return Ok(true);
            }
        }
        // 3) 目标缺席
        if target_digest.is_none() {
            if remove_like && tx.phase == "committed" {
                // 移除已提交完成（基线已是「不存在」）：清理残留旧副本
                cleanup(&tx_root)?;
                self.store.set_transaction_phase(&tx.id, "done")?;
                return Ok(true);
            }
            match (&old, old_digest.as_deref(), expected_old) {
                // 旧目录已移出且内容可核验 → 恢复旧版本
                (Some(old_path), Some(d), Some(e)) if d == e => {
                    fsops::rename_in_place(old_path, &target)?;
                    abort_item("同步在提交中中断，已恢复旧版本");
                    cleanup(&tx_root)?;
                    self.store.set_transaction_phase(&tx.id, "aborted")?;
                    return Ok(true);
                }
                // 新增类事务中断于改名前：从未有旧内容，仅清理暂存
                _ if expected_old.is_none() && old_digest.is_none() => {
                    abort_item("同步在提交前中断，目标未被创建");
                    cleanup(&tx_root)?;
                    self.store.set_transaction_phase(&tx.id, "aborted")?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        // 4) 其余均为未知状态：不覆盖未知内容，转人工
        Err(AppError::new(
            ErrorCode::RecoveryPending,
            format!(
                "目标处于未知状态（阶段 {}），已保留现场等待人工处理：{}",
                tx.phase, tx.target_path
            ),
        ))
    }

    // ------------------------------------------------------------------
    // 历史恢复（§6.5/§7.4）
    // ------------------------------------------------------------------

    /// 生成恢复计划：把当前目标与「待撤销操作完成后的已知内容」比较；
    /// 存在后续修改显示冲突（restore_drift），用户明确选择后才覆盖（AC-23）。
    pub fn create_restore_plan(
        &self,
        input: &CreateRestorePlanInput,
    ) -> AppResult<SyncPlan> {
        let task = self.store.get_task(&input.task_id)?;
        let task_items = self.store.list_task_items(&task.id)?;
        let mut items: Vec<PlanItem> = Vec::new();
        let mut groups: Vec<PlanGroupMeta> = Vec::new();
        let mut library_id: Option<String> = None;

        for item_id in &input.item_ids {
            let item = task_items
                .iter()
                .find(|i| &i.id == item_id)
                .ok_or_else(|| AppError::not_found("历史任务项", item_id))?;
            let mapping_id = item
                .mapping_id
                .clone()
                .ok_or_else(|| AppError::new(ErrorCode::ValidationFailed, "该历史项没有关联映射"))?;
            let mapping = self.store.get_mapping(&mapping_id)?;
            library_id.get_or_insert_with(|| mapping.library_id.clone());
            let physical = self.store.get_physical_target(&mapping.physical_target_id)?;
            let target_dir = PathBuf::from(&physical.canonical_path).join(&mapping.target_dir_name);
            if !groups
                .iter()
                .any(|g| g.physical_target_id == physical.id)
            {
                groups.push(PlanGroupMeta {
                    physical_target_id: physical.id.clone(),
                    target_path: physical.canonical_path.clone(),
                    shared_with_tools: self
                        .store
                        .targets_on_physical(&physical.id)
                        .map(|ts| ts.iter().map(|t| t.display_name.clone()).collect())
                        .unwrap_or_default(),
                });
            }

            let Some(snapshot_id) = item.snapshot_id.clone() else {
                items.push(self.blocked_restore_item(
                    &mapping, &target_dir, item, "该历史项没有可恢复的备份快照",
                ));
                continue;
            };
            let snapshot = self.store.get_snapshot(&snapshot_id)?;
            let intact = self.backups.verify(&snapshot)?;
            if !intact {
                items.push(self.blocked_restore_item(
                    &mapping, &target_dir, item, "备份已过期或损坏，当前目标不会被改动",
                ));
                continue;
            }

            let to_absent = !snapshot.existed_before;
            let current_t = scanner::digest_directory(&target_dir)?;
            // 恢复点的已知内容 = 该项提交后的摘要（§7.4）
            let drift = match (&current_t, &item.post_digest) {
                (None, None) => false,
                (Some((t, _)), Some(post)) => t != post,
                (None, Some(_)) => true,
                (Some(_), None) => true,
            };
            let baseline = self.store.get_baseline(&mapping.id)?;

            // 文件变化：快照清单 vs 当前目标清单
            let snapshot_manifest: Vec<scanner::FileEntry> = if to_absent {
                Vec::new()
            } else {
                let dir = self
                    .backups
                    .snapshot_dir(snapshot.backup_dir.as_deref().unwrap());
                scanner::digest_directory(&dir)?.map(|(_, m)| m).unwrap_or_default()
            };
            let cur_manifest = current_t
                .as_ref()
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            let mut file_changes = crate::planner::diff_manifests_pub(
                &snapshot_manifest,
                &cur_manifest,
            );
            if to_absent {
                file_changes = cur_manifest
                    .iter()
                    .map(|t| FileChange {
                        kind: "delete".into(),
                        rel_path: t.rel_path.clone(),
                        bytes: t.size,
                        source_digest: None,
                        target_digest: Some(t.sha256.clone()),
                        diff_previewable: false,
                    })
                    .collect();
            }

            let (selected, conflict) = if drift {
                (
                    false,
                    Some(ConflictInfo {
                        kind: "restore_drift".to_string(),
                        message: "恢复点之后目标又有修改；恢复将先保存当前快照，确认后才覆盖".to_string(),
                        available_choices: vec![
                            ConflictChoice::KeepTarget,
                            ConflictChoice::OverwriteWithSource,
                        ],
                    }),
                )
            } else {
                (true, None)
            };
            items.push(PlanItem {
                item_id: uuid::Uuid::new_v4().to_string(),
                mapping_id: mapping.id.clone(),
                skill_id: mapping.skill_id.clone(),
                skill_name: item.skill_name.clone(),
                rel_path: mapping.target_dir_name.clone(),
                action: PlanAction::Restore,
                file_changes,
                excluded_by_ignore: Vec::new(),
                needs_backup: true, // 恢复前对当前目标再次快照（§6.5）
                target_path: target_dir.to_string_lossy().to_string(),
                state_before: MatrixCellState::Synced,
                conflict,
                blocked_reason: None,
                selected,
                decision: None,
                source_digest: snapshot.digest.clone(),
                target_digest: current_t.map(|(d, _)| d),
                baseline_digest: baseline.map(|b| b.digest),
                restore: Some(RestoreSpec {
                    snapshot_id: snapshot.id.clone(),
                    to_absent,
                }),
            });
        }

        let library_id = library_id.ok_or_else(|| {
            AppError::new(ErrorCode::ValidationFailed, "没有可恢复的历史项")
        })?;
        let library = self.store.get_library(&library_id)?;
        let fingerprint =
            crate::planner::compute_fingerprint(library.config_version, PlanOperation::Restore, &items);
        let plan_id = uuid::Uuid::new_v4().to_string();
        self.store.insert_plan(
            &plan_id,
            &library_id,
            PlanOperation::Restore,
            &fingerprint,
            &items,
            &groups,
            library.config_version,
        )?;
        let row = self.store.get_plan(&plan_id)?;
        self.store.plan_view(&row)
    }

    fn blocked_restore_item(
        &self,
        mapping: &crate::storage::MappingRow,
        target_dir: &Path,
        item: &crate::storage::TaskItemRow,
        reason: &str,
    ) -> PlanItem {
        PlanItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            mapping_id: mapping.id.clone(),
            skill_id: mapping.skill_id.clone(),
            skill_name: item.skill_name.clone(),
            rel_path: mapping.target_dir_name.clone(),
            action: PlanAction::Blocked,
            file_changes: Vec::new(),
            excluded_by_ignore: Vec::new(),
            needs_backup: false,
            target_path: target_dir.to_string_lossy().to_string(),
            state_before: MatrixCellState::Synced,
            conflict: None,
            blocked_reason: Some(reason.to_string()),
            selected: false,
            decision: None,
            source_digest: None,
            target_digest: None,
            baseline_digest: None,
            restore: None,
        }
    }
}
