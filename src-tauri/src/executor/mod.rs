//! 事务执行器（需求 §7.3、§8.4、§8.5）。
//!
//! 单技能事务流程（与需求 §8.4 流程图一致）：
//!   校验计划与路径 → 同卷暂存 → 校验暂存 → 备份旧内容并登记事务 → 复核摘要
//!   → 旧目录改名移出 → 暂存改名到目标 → 核验目标并提交基线 → 登记成功并清理
//!
//! 保证：
//! - 备份失败不得继续覆盖（AC-19）；提交前复核源/目标摘要（§8.4 E）
//! - 同一物理目标串行，不同物理目标最多 2 个并行；单项失败隔离
//! - 取消停止未开始项；在途单元先完成或回滚（AC-21）
//! - 每个关键阶段写入持久化事务日志（DB + journals/<事务ID>.json）

use crate::backup::BackupStore;
use crate::contract::*;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::fsops;
use crate::windows_paths;
use crate::planner;
use crate::scanner;
use crate::storage::{final_task_status, LibraryRow, Store, TransactionRow};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 事件出口：核心库不依赖 Tauri 运行时，由命令层注入实现。
pub trait EventSink: Send + Sync {
    fn emit_json(&self, event: &str, payload: serde_json::Value);
    fn next_seq(&self) -> u64;
}

/// 空事件出口（测试用）。
pub struct NullSink;
impl EventSink for NullSink {
    fn emit_json(&self, _event: &str, _payload: serde_json::Value) {}
    fn next_seq(&self) -> u64 {
        0
    }
}

pub struct Runner<'a> {
    pub store: &'a Store,
    pub backups: &'a BackupStore,
    pub data_dir: &'a Path,
    pub sink: &'a dyn EventSink,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Journal {
    id: String,
    phase: String,
    task_item_id: String,
    mapping_id: String,
    skill_dir_name: String,
    staging_path: Option<String>,
    old_path: Option<String>,
    target_path: String,
    backup_id: Option<String>,
    expected_source_digest: Option<String>,
    expected_old_digest: Option<String>,
    updated_at: String,
}

impl<'a> Runner<'a> {
    fn journal_path(&self, tx_id: &str) -> PathBuf {
        self.data_dir.join("journals").join(format!("{tx_id}.json"))
    }

    fn write_journal(&self, j: &Journal) -> AppResult<()> {
        let dir = self.data_dir.join("journals");
        std::fs::create_dir_all(&dir).map_err(AppError::from)?;
        let tmp = dir.join(format!("{}.tmp", j.id));
        std::fs::write(&tmp, serde_json::to_vec_pretty(j)?).map_err(AppError::from)?;
        std::fs::rename(&tmp, self.journal_path(&j.id)).map_err(AppError::from)?;
        self.store.upsert_transaction(&TransactionRow {
            id: j.id.clone(),
            task_item_id: Some(j.task_item_id.clone()),
            mapping_id: Some(j.mapping_id.clone()),
            physical_target_id: None,
            skill_dir_name: j.skill_dir_name.clone(),
            phase: j.phase.clone(),
            staging_path: j.staging_path.clone(),
            old_path: j.old_path.clone(),
            target_path: j.target_path.clone(),
            backup_id: j.backup_id.clone(),
            expected_source_digest: j.expected_source_digest.clone(),
            expected_old_digest: j.expected_old_digest.clone(),
        })
    }

    fn progress(&self, task_id: &str, phase: &str, idx: u32, count: u32, item: Option<&PlanItem>, msg: Option<String>) {
        self.sink.emit_json(
            EVENT_SYNC_PROGRESS,
            serde_json::to_value(SyncProgressEvent {
                task_id: task_id.to_string(),
                seq: self.sink.next_seq(),
                phase: phase.to_string(),
                item_index: idx,
                item_count: count,
                item_id: item.map(|i| i.item_id.clone()),
                skill_name: item.map(|i| i.skill_name.clone()),
                bytes_done: None,
                bytes_total: item.map(|i| {
                    i.file_changes
                        .iter()
                        .filter(|f| f.kind != "delete")
                        .map(|f| f.bytes)
                        .sum()
                }),
                message: msg,
            })
            .unwrap_or_default(),
        );
    }

    /// 执行一个已生成并确认的同步计划（§7.3）。
    pub fn run_plan(
        &self,
        plan_id: &str,
        plan_version: i64,
        task_id: &str,
        trigger: TaskTrigger,
        cancel: Arc<AtomicBool>,
    ) -> AppResult<()> {
        let started = std::time::Instant::now();
        self.store.update_task_status(task_id, TaskStatus::Running)?;
        let finish = |status: TaskStatus, err: Option<&AppError>| -> AppResult<()> {
            let items = self.store.list_task_items(task_id)?;
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
            let cancelled = items.iter().any(|i| i.status == TaskItemStatus::Cancelled);
            let recovery = items
                .iter()
                .any(|i| i.status == TaskItemStatus::RecoveryPending);
            let status = if status == TaskStatus::Running {
                final_task_status(&counts, cancelled, recovery)
            } else {
                status
            };
            self.store.set_task_counts(task_id, &counts)?;
            self.store.set_task_error(task_id, err)?;
            self.store.update_task_status(task_id, status)?;
            // 收拢 WAL，缩小崩溃损失窗口
            self.store.checkpoint();
            // 追加人类可读操作日志到「文档\SkillDock」（独立于数据库存续）
            let task = self.store.get_task(task_id).ok();
            let library_name = task
                .as_ref()
                .and_then(|t| t.library_id.as_deref())
                .and_then(|id| self.store.get_library(id).ok())
                .map(|l| l.display_name);
            crate::oplog::append(&crate::oplog::OpLogEntry {
                time: crate::storage::now_iso(),
                task_id: task_id.to_string(),
                kind: task
                    .as_ref()
                    .and_then(|t| {
                        serde_json::to_value(t.kind).ok()?.as_str().map(String::from)
                    })
                    .unwrap_or_else(|| "sync".into()),
                trigger: serde_json::to_value(trigger)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| "manual".into()),
                status: status.as_str().to_string(),
                library: library_name,
                duration_ms: Some(started.elapsed().as_millis() as u64),
                counts: counts.clone(),
                items: items
                    .iter()
                    .map(|i| crate::oplog::OpLogItem {
                        skill_name: i.skill_name.clone(),
                        action: format!("{:?}", i.action),
                        status: i.status.as_str().to_string(),
                        target_path: i.target_path.clone(),
                        error: i.error.as_ref().map(|e| e.message.clone()),
                    })
                    .collect(),
            });
            self.sink.emit_json(
                EVENT_SYNC_COMPLETED,
                serde_json::to_value(SyncCompletedEvent {
                    task_id: task_id.to_string(),
                    seq: self.sink.next_seq(),
                    status,
                    counts,
                    duration_ms: started.elapsed().as_millis() as u64,
                })
                .unwrap_or_default(),
            );
            Ok(())
        };

        // 执行前复核（§7.2）：失败则整任务失败，不写任何目标
        let plan = match planner::validate_for_execute(self.store, plan_id, plan_version) {
            Ok(p) => p,
            Err(e) => {
                finish(TaskStatus::Failed, Some(&e))?;
                return Err(e);
            }
        };

        // 未决冲突不得进入执行（契约不变量 1）
        if let Some(item) = plan
            .items
            .iter()
            .find(|i| i.selected && i.conflict.is_some() && i.decision.is_none())
        {
            let e = AppError::new(
                ErrorCode::ConflictUnresolved,
                format!("「{}」存在未处理的冲突，请先处理后再执行", item.skill_name),
            );
            finish(TaskStatus::Failed, Some(&e))?;
            return Err(e);
        }

        let library = self.store.get_library(&plan.library_id)?;
        let actionable: Vec<&PlanItem> = plan
            .items
            .iter()
            .filter(|i| {
                i.selected
                    && !matches!(i.action, PlanAction::Skip | PlanAction::Blocked)
            })
            .collect();

        // 登记全部计划项为任务项（跳过/冲突也入列：跳过与冲突单独统计，§11.1）
        let mut seq = 0i64;
        let mut task_item_ids: HashMap<String, String> = HashMap::new();
        for item in &plan.items {
            seq += 1;
            let task_item_id = uuid::Uuid::new_v4().to_string();
            self.store.insert_task_item(
                &task_item_id,
                task_id,
                Some(&item.mapping_id),
                &item.skill_name,
                &item.target_path,
                item.action,
                item.file_changes
                    .iter()
                    .filter(|f| f.kind != "delete")
                    .map(|f| f.bytes)
                    .sum(),
                seq,
            )?;
            if !actionable.iter().any(|a| a.item_id == item.item_id) {
                let (status, msg) = match item.action {
                    PlanAction::Blocked => (
                        TaskItemStatus::Skipped,
                        item.blocked_reason.clone().unwrap_or_else(|| "阻塞项".into()),
                    ),
                    _ if item.conflict.is_some() => (
                        TaskItemStatus::ConflictPending,
                        item.conflict
                            .as_ref()
                            .map(|c| c.message.clone())
                            .unwrap_or_default(),
                    ),
                    _ => (
                        TaskItemStatus::Skipped,
                        match item.state_before {
                            MatrixCellState::Synced => "内容已一致，无动作".to_string(),
                            MatrixCellState::SourceRemoved => "源已移除，目标保留".to_string(),
                            MatrixCellState::TargetDeleted => "目标已删除，保留删除状态".to_string(),
                            MatrixCellState::Paused => "映射已暂停".to_string(),
                            _ => "未选中或无需处理".to_string(),
                        },
                    ),
                };
                let err = AppError::new(ErrorCode::ValidationFailed, msg).retryable(false);
                self.store.update_task_item_status(&task_item_id, status, Some(&err))?;
            }
            task_item_ids.insert(item.item_id.clone(), task_item_id);
        }

        // 按物理目标分组；同目标串行，跨目标最多 2 路并行（§7.3）
        let mut groups: HashMap<String, Vec<&PlanItem>> = HashMap::new();
        for item in actionable {
            let mapping = self.store.get_mapping(&item.mapping_id)?;
            groups
                .entry(mapping.physical_target_id)
                .or_default()
                .push(item);
        }
        let queue = Mutex::new(VecDeque::from(
            groups.into_iter().collect::<Vec<(String, Vec<&PlanItem>)>>(),
        ));
        let item_count = plan.items.len() as u32;
        let item_index = Mutex::new(0u32);
        let worker = || loop {
            let group = queue
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop_front();
            let Some((_physical_id, items)) = group else { break };
            let mut cascade: Option<AppError> = None;
            for item in items {
                let idx = {
                    let mut g = item_index.lock().unwrap_or_else(|e| e.into_inner());
                    *g += 1;
                    *g
                };
                let task_item_id = task_item_ids[&item.item_id].clone();
                if cancel.load(Ordering::Relaxed) {
                    let e = AppError::new(ErrorCode::ValidationFailed, "已取消：该项未开始").retryable(false);
                    let _ = self
                        .store
                        .update_task_item_status(&task_item_id, TaskItemStatus::Cancelled, Some(&e));
                    continue;
                }
                if let Some(cause) = &cascade {
                    // 目标级共性错误（权限/磁盘）：暂停该目标余下操作（§7.3）
                    let e = AppError::new(
                        cause.code,
                        format!("该目标此前发生共性错误，余下操作暂停：{}", cause.message),
                    )
                    .retryable(false);
                    let _ = self
                        .store
                        .update_task_item_status(&task_item_id, TaskItemStatus::Skipped, Some(&e));
                    continue;
                }
                match self.execute_item(item, &task_item_id, task_id, idx, item_count, &library) {
                    Ok(()) => {}
                    Err(e) => {
                        if matches!(e.code, ErrorCode::PermissionDenied | ErrorCode::DiskFull) {
                            cascade = Some(e.clone());
                        }
                        let status = if e.code == ErrorCode::RecoveryPending {
                            TaskItemStatus::RecoveryPending
                        } else {
                            TaskItemStatus::Failed
                        };
                        let _ = self.store.update_task_item_status(&task_item_id, status, Some(&e));
                    }
                }
            }
        };
        std::thread::scope(|s| {
            let h1 = s.spawn(worker);
            let h2 = s.spawn(worker);
            let _ = h1.join();
            let _ = h2.join();
        });

        self.store.set_plan_status(plan_id, PlanStatus::Executed)?;
        let _ = self.backups.sweep(self.store);
        finish(TaskStatus::Running, None)?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 单技能事务
    // ------------------------------------------------------------------

    fn execute_item(
        &self,
        item: &PlanItem,
        task_item_id: &str,
        task_id: &str,
        idx: u32,
        count: u32,
        library: &LibraryRow,
    ) -> AppResult<()> {
        self.progress(task_id, "staging", idx, count, Some(item), None);
        self.store
            .update_task_item_status(task_item_id, TaskItemStatus::Running, None)?;
        let mapping = self.store.get_mapping(&item.mapping_id)?;
        let physical = self.store.get_physical_target(&mapping.physical_target_id)?;
        let physical_dir = PathBuf::from(&physical.canonical_path);
        let target_dir = physical_dir.join(&mapping.target_dir_name);

        // Adopt：内容已一致，只建立/刷新基线，不复制（§8.2 双方一致行）
        if item.action == PlanAction::Adopt {
            let t = scanner::digest_directory(&target_dir)?
                .ok_or_else(|| AppError::new(ErrorCode::TargetDrift, "接管核验时目标目录不存在"))?;
            let s = item.source_digest.clone().unwrap_or_default();
            if t.0 != s {
                return Err(AppError::new(
                    ErrorCode::TargetDrift,
                    "接管核验失败：目标与源内容已不一致",
                ));
            }
            self.store
                .set_baseline(&mapping.id, Some(&s), &t.1, Some(task_id))?;
            self.store.update_task_item_digests(
                task_item_id,
                item.target_digest.as_deref(),
                Some(&s),
                0,
            )?;
            self.store
                .update_task_item_status(task_item_id, TaskItemStatus::Success, None)?;
            self.progress(task_id, "item_done", idx, count, Some(item), None);
            return Ok(());
        }

        let tx_id = uuid::Uuid::new_v4().to_string();
        let tx_parent = fsops::sibling_transaction_root(&physical_dir);
        let tx_root = tx_parent.join(&tx_id);
        let staging = tx_root.join("staging");
        let old_dir = tx_root.join("old");

        let mut journal = Journal {
            id: tx_id.clone(),
            phase: "init".to_string(),
            task_item_id: task_item_id.to_string(),
            mapping_id: mapping.id.clone(),
            skill_dir_name: mapping.target_dir_name.clone(),
            staging_path: None,
            old_path: None,
            target_path: target_dir.to_string_lossy().to_string(),
            backup_id: None,
            expected_source_digest: item.source_digest.clone(),
            expected_old_digest: item.target_digest.clone(),
            updated_at: crate::storage::now_iso(),
        };
        self.write_journal(&journal)?;

        let result = self.transaction_body(
            item, task_item_id, task_id, idx, count, library,
            &physical_dir, &target_dir, &tx_root, &staging, &old_dir, &mut journal,
        );
        match result {
            Ok(()) => {
                // 成功状态已随 commit_item_success 一并落库
                self.progress(task_id, "item_done", idx, count, Some(item), None);
                Ok(())
            }
            Err(e) => {
                // 回滚到可解释状态；未知内容保留现场（§8.4.5）
                if let Err(rb) = self.rollback(&mut journal, &target_dir, &tx_root, &old_dir) {
                    let _ = self.store.set_transaction_phase(&tx_id, "manual");
                    self.sink.emit_json(
                        EVENT_RECOVERY_REQUIRED,
                        serde_json::to_value(RecoveryRequiredEvent {
                            task_id: Some(task_id.to_string()),
                            seq: self.sink.next_seq(),
                            transaction_id: tx_id.clone(),
                            target_path: target_dir.to_string_lossy().to_string(),
                            skill_name: Some(item.skill_name.clone()),
                            reason: format!("事务回滚失败，已保留现场等待人工处理：{rb}"),
                            recoverable: "manual".to_string(),
                        })
                        .unwrap_or_default(),
                    );
                    return Err(AppError::new(
                        ErrorCode::RecoveryPending,
                        format!("「{}」提交失败且自动回滚受阻，已保留现场：{rb}", item.skill_name),
                    ));
                }
                Err(e)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn transaction_body(
        &self,
        item: &PlanItem,
        task_item_id: &str,
        task_id: &str,
        idx: u32,
        count: u32,
        library: &LibraryRow,
        physical_dir: &Path,
        target_dir: &Path,
        tx_root: &Path,
        staging: &Path,
        old_dir: &Path,
        journal: &mut Journal,
    ) -> AppResult<()> {
        let needs_content = !matches!(
            item.action,
            PlanAction::Remove | PlanAction::Adopt
        ) && !item.restore.as_ref().map(|r| r.to_absent).unwrap_or(false);

        // 0. 事务空间预检（§8.4.1/8.4.2）
        let stage_bytes: u64 = item
            .file_changes
            .iter()
            .filter(|f| f.kind != "delete")
            .map(|f| f.bytes)
            .sum();
        if let Err(e) = self.prepare_tx_space(physical_dir, tx_root, stage_bytes) {
            journal.phase = "aborted".into();
            let _ = self.write_journal(journal);
            return Err(e);
        }

        // 1. 暂存（同卷）并校验（§8.4 B/C）：只复制纳入分发清单的文件（§4.3）；
        //    单遍复制+逐文件 sha256 核验，摘要直接由已核验清单计算（不再二次读取暂存）
        if needs_content {
            let (source_dir, manifest) = self.content_manifest(item, library)?;
            let expected = item
                .source_digest
                .clone()
                .ok_or_else(|| AppError::internal("计划项缺少源摘要"))?;
            let (verified, _bytes) = fsops::copy_manifest(&source_dir, &manifest, staging)?;
            let staged = scanner::digest_manifest(&verified);
            if staged != expected {
                return Err(AppError::new(
                    ErrorCode::IoError,
                    format!("暂存内容校验失败：期望 {expected}，实际 {staged}"),
                ));
            }
            journal.staging_path = Some(staging.to_string_lossy().to_string());
            journal.phase = "staged".into();
            journal.updated_at = crate::storage::now_iso();
            self.write_journal(journal)?;
        }

        // 2. 备份旧内容并登记（§8.4 D；备份失败则该项不执行，AC-19）
        self.progress(task_id, "backup", idx, count, Some(item), None);
        let snapshot = self.backups.create_snapshot(
            self.store,
            Some(task_item_id),
            Some(&item.mapping_id),
            target_dir,
        )?;
        journal.backup_id = Some(snapshot.id.clone());
        journal.expected_old_digest = snapshot.digest.clone();
        journal.phase = "backed_up".into();
        journal.updated_at = crate::storage::now_iso();
        self.write_journal(journal)?;

        // 3. 提交前复核源与目标摘要（§8.4 E）
        let cur_t = scanner::digest_directory(target_dir)?.map(|(d, _)| d);
        if cur_t != item.target_digest {
            return Err(AppError::new(
                ErrorCode::TargetDrift,
                "提交前复核发现目标内容已变化，中止该项",
            ));
        }
        if needs_content {
            // 用与计划一致的口径复核（含忽略规则；恢复项复核快照）
            let (_, manifest) = self.content_manifest(item, library)?;
            let cur_s = scanner::digest_manifest(&manifest);
            if Some(cur_s) != item.source_digest {
                return Err(AppError::new(
                    ErrorCode::PlanStale,
                    "提交前复核发现源内容已变化，请刷新预览",
                ));
            }
        }

        // 4. 旧目录改名移出 → 暂存改名到目标（§8.4 F/G）
        self.progress(task_id, "committing", idx, count, Some(item), None);
        let target_exists = target_dir.exists();
        if target_exists {
            fsops::rename_in_place(target_dir, old_dir)?;
            journal.old_path = Some(old_dir.to_string_lossy().to_string());
            journal.phase = "old_moved_out".into();
            journal.updated_at = crate::storage::now_iso();
            self.write_journal(journal)?;
        }
        if needs_content {
            fsops::rename_in_place(staging, target_dir)?;
            journal.staging_path = None;
            journal.phase = "new_moved_in".into();
            journal.updated_at = crate::storage::now_iso();
            self.write_journal(journal)?;
        }

        // 5. 核验目标并提交基线（§8.4 H）
        self.progress(task_id, "verifying", idx, count, Some(item), None);
        let new_digest = scanner::digest_directory(target_dir)?;
        let expect_new: Option<String> = if matches!(item.action, PlanAction::Remove)
            || item.restore.as_ref().map(|r| r.to_absent).unwrap_or(false)
        {
            None
        } else {
            item.source_digest.clone()
        };
        let ok = match (&new_digest, &expect_new) {
            (None, None) => true,
            (Some((d, _)), Some(e)) => d == e,
            _ => false,
        };
        if !ok {
            return Err(AppError::new(
                ErrorCode::IoError,
                "目标核验失败：提交后内容与计划不一致",
            ));
        }
        let (post_digest, manifest) = match &new_digest {
            Some((d, m)) => (Some(d.clone()), m.clone()),
            None => (None, Vec::new()),
        };
        journal.phase = "committed".into();
        journal.updated_at = crate::storage::now_iso();
        self.write_journal(journal)?;

        // 6. 清理事务（§8.4 I）：旧目录内容已由备份持有
        self.progress(task_id, "cleanup", idx, count, Some(item), None);
        if tx_root.exists() {
            std::fs::remove_dir_all(tx_root).map_err(AppError::from)?;
        }
        let _ = std::fs::remove_file(self.journal_path(&journal.id));
        journal.phase = "done".into();

        // 恢复成功后暂停相关映射的自动同步（§6.5）
        if item.action == PlanAction::Restore {
            self.store
                .set_mapping_paused(&item.mapping_id, Some("restored"))?;
        }
        let bytes: u64 = item
            .file_changes
            .iter()
            .filter(|f| f.kind != "delete")
            .map(|f| f.bytes)
            .sum();
        // 基线 + 项状态 + 事务完成：合并为一个 DB 事务（§13 性能）
        self.store.commit_item_success(
            &item.mapping_id,
            post_digest.as_deref(),
            &manifest,
            task_id,
            task_item_id,
            post_digest.as_deref(),
            bytes,
            &journal.id,
        )?;
        Ok(())
    }

    /// 事务内容来源与分发清单：恢复取快照目录；其余按忽略规则实时扫描源库技能目录。
    fn content_manifest(
        &self,
        item: &PlanItem,
        library: &LibraryRow,
    ) -> AppResult<(PathBuf, Vec<scanner::FileEntry>)> {
        if let Some(restore) = &item.restore {
            if restore.to_absent {
                return Err(AppError::internal("恢复为「不存在」不需要内容来源"));
            }
            let snap = self.store.get_snapshot(&restore.snapshot_id)?;
            let dir = snap.backup_dir.ok_or_else(|| {
                AppError::new(ErrorCode::BackupFailed, "恢复快照缺少备份内容")
            })?;
            let path = self.backups.snapshot_dir(&dir);
            let Some((_, manifest)) = scanner::digest_directory(&path)? else {
                return Err(AppError::new(
                    ErrorCode::BackupFailed,
                    "恢复快照缺失或已过期，阻止恢复（当前目标不会被删除）",
                ));
            };
            return Ok((path, manifest));
        }
        let source_root = library.source_root.clone().ok_or_else(|| {
            AppError::new(ErrorCode::ValidationFailed, "技能库未选定源根")
        })?;
        let source_root = PathBuf::from(source_root);
        let ignore = scanner::build_globset(&library.ignore_patterns)?;
        // 单技能直扫（不遍历源根其它目录；§13 性能）
        let skill = scanner::scan_single_skill(&source_root, &item.rel_path, Some(&ignore))?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::PlanStale,
                    format!("源技能目录已消失：{}", item.rel_path),
                )
            })?;
        Ok((skill.abs_path, skill.manifest))
    }

    /// 事务空间预检（§8.4.1/8.4.2）：
    /// - 事务目录放在技能集合目录之外的同卷兄弟目录
    /// - 保留名称已存在未知内容则阻止使用
    /// - 父目录写入与重命名权限、空间预检
    fn prepare_tx_space(&self, physical_dir: &Path, tx_root: &Path, stage_bytes: u64) -> AppResult<()> {
        // 目标集合目录可能尚不存在（will_create）：先创建
        std::fs::create_dir_all(physical_dir).map_err(AppError::from)?;
        if windows_paths::contains_reparse_point(physical_dir)? {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "目标路径包含符号链接/目录联接，阻止执行",
            ));
        }
        let tx_parent = fsops::sibling_transaction_root(physical_dir);
        if tx_parent.exists() {
            let known = self.store.known_transaction_dirs()?;
            let read = std::fs::read_dir(&tx_parent).map_err(|e| {
                AppError::from(e).with_context(serde_json::json!({ "path": tx_parent.to_string_lossy() }))
            })?;
            for entry in read {
                let entry = entry.map_err(AppError::from)?;
                let name = entry.file_name().to_string_lossy().to_string();
                if !known.iter().any(|k| k == &name) {
                    return Err(AppError::new(
                        ErrorCode::RecoveryPending,
                        format!(
                            "事务目录 {} 存在未知内容 `{name}`，已阻止使用；请检查或清理",
                            tx_parent.display()
                        ),
                    ));
                }
            }
        }
        std::fs::create_dir_all(tx_root).map_err(|e| {
            AppError::from(e).with_context(serde_json::json!({ "path": tx_root.to_string_lossy() }))
        })?;
        // 写入/重命名权限与空间预检（§8.5：考虑暂存新旧版本同时存在的峰值）
        let probe = tx_root.join(".probe");
        std::fs::write(&probe, b"ok").map_err(AppError::from)?;
        std::fs::rename(&probe, tx_root.join(".probe2")).map_err(AppError::from)?;
        std::fs::remove_file(tx_root.join(".probe2")).map_err(AppError::from)?;
        if let Some(free) = crate::windows_paths::free_space(tx_root) {
            if free < stage_bytes {
                return Err(AppError::new(
                    ErrorCode::DiskFull,
                    format!("事务暂存空间不足：需要 {stage_bytes} 字节，可用 {free} 字节"),
                ));
            }
        }
        Ok(())
    }

    /// 回滚到可解释状态（§8.4.5）；发现未知内容返回错误，由调用方转入人工恢复。
    fn rollback(
        &self,
        journal: &mut Journal,
        target_dir: &Path,
        tx_root: &Path,
        old_dir: &Path,
    ) -> AppResult<()> {
        let phase = journal.phase.clone();
        let unknown_content = AppError::new(
            ErrorCode::RecoveryPending,
            "发现未知第三方内容，停止自动回滚并保留现场",
        );
        match phase.as_str() {
            "init" | "staged" | "backed_up" => {
                // 正式目录尚未变更；若目标已不在原位说明 rename 已发生，按 old_moved_out 处理
                if !target_dir.exists() && old_dir.exists() {
                    fsops::rename_in_place(old_dir, target_dir)?;
                    self.verify_restored(target_dir, journal.expected_old_digest.as_deref())?;
                }
                if tx_root.exists() {
                    std::fs::remove_dir_all(tx_root).map_err(AppError::from)?;
                }
            }
            "old_moved_out" => {
                if target_dir.exists() {
                    return Err(unknown_content);
                }
                if old_dir.exists() {
                    fsops::rename_in_place(old_dir, target_dir)?;
                    self.verify_restored(target_dir, journal.expected_old_digest.as_deref())?;
                }
                if tx_root.exists() {
                    std::fs::remove_dir_all(tx_root).map_err(AppError::from)?;
                }
            }
            "new_moved_in" => {
                // 新内容已就位但核验失败：移出新目录，恢复旧目录
                if target_dir.exists() {
                    let bad_new = tx_root.join("rejected_new");
                    fsops::rename_in_place(target_dir, &bad_new)?;
                }
                if old_dir.exists() {
                    fsops::rename_in_place(old_dir, target_dir)?;
                    self.verify_restored(target_dir, journal.expected_old_digest.as_deref())?;
                }
                if tx_root.exists() {
                    std::fs::remove_dir_all(tx_root).map_err(AppError::from)?;
                }
            }
            _ => {
                // committed/done/aborted/manual 不回滚
            }
        }
        if tx_root.exists() {
            // 保守：仍有残留则不删
        }
        let _ = std::fs::remove_file(self.journal_path(&journal.id));
        journal.phase = "aborted".into();
        journal.updated_at = crate::storage::now_iso();
        self.store.set_transaction_phase(&journal.id, "aborted")?;
        Ok(())
    }

    fn verify_restored(&self, target_dir: &Path, expected: Option<&str>) -> AppResult<()> {
        let actual = scanner::digest_directory(target_dir)?.map(|(d, _)| d);
        let matches = match (&actual, expected) {
            (None, None) => true,
            (Some(a), Some(e)) => a == e,
            _ => false,
        };
        if matches {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::RecoveryPending,
                "回滚后内容摘要与事务记录不符，保留现场",
            ))
        }
    }
}
