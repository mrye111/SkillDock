//! 隔离临时目录中的集成测试（需求 §14：不得触碰真实 Agent 技能目录）。
//!
//! 覆盖主闭环「扫描 → 预览 → 同步到自定义目录 → 校验结果」及
//! 冲突、备份、崩溃恢复、历史恢复等验收场景（AC 编号见各测试注释）。

use skilldock_lib::backup::BackupStore;
use skilldock_lib::contract::*;
use skilldock_lib::error::ErrorCode;
use skilldock_lib::executor::{NullSink, Runner};
use skilldock_lib::planner;
use skilldock_lib::recovery::Recovery;
use skilldock_lib::scanner::{self, ScanOptions};
use skilldock_lib::storage::{Store, TransactionRow};
use skilldock_lib::windows_paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 测试环境
// ---------------------------------------------------------------------------

struct Env {
    _tmp: TempDir,
    data: PathBuf,
    src: PathBuf,
    dst: PathBuf,
    store: Store,
    backups: BackupStore,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let data = root.join("appdata");
        let src = root.join("source-lib");
        let dst = root.join("agent-target").join("skills");
        std::fs::create_dir_all(&src).unwrap();
        let store = Store::open(&data).unwrap();
        let backups = BackupStore::new(&data).unwrap();
        Self {
            _tmp: tmp,
            data,
            src,
            dst,
            store,
            backups,
        }
    }

    /// 写一个合法技能：SKILL.md（name 与目录名一致）+ 附加文件
    fn write_skill(&self, dir_name: &str, desc: &str, extra: &[&str]) {
        let dir = self.src.join(dir_name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {dir_name}\ndescription: {desc}\n---\n\n# {dir_name} 正文\n"),
        )
        .unwrap();
        for (i, content) in extra.iter().enumerate() {
            let rel = format!("assets/file-{i}.txt");
            let p = dir.join(&rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
    }

    fn register_and_scan(&self) -> String {
        let library_id = uuid::Uuid::new_v4().to_string();
        let canonical = windows_paths::canonicalize(&self.src).unwrap();
        let identity = windows_paths::fold_case(&canonical);
        self.store
            .insert_library(
                &library_id,
                "测试库",
                &canonical.to_string_lossy(),
                Some(&canonical.to_string_lossy()),
                Some(&identity),
                "collection",
                None,
            )
            .unwrap();
        self.rescan(&library_id);
        library_id
    }

    fn rescan(&self, library_id: &str) {
        let lib = self.store.get_library(library_id).unwrap();
        let ignore = scanner::build_globset(&lib.ignore_patterns).unwrap();
        let scanned = scanner::scan_source_root(
            &windows_paths::canonicalize(&self.src).unwrap(),
            &ScanOptions {
                ignore: Some(&ignore),
                skill_filter: None,
                on_dir: None,
            },
        )
        .unwrap();
        self.store.apply_scan(library_id, &scanned).unwrap();
    }

    /// 自定义物理目标（不经命令层，直接落库）
    fn add_target(&self) -> String {
        let identity = windows_paths::fold_case(&self.dst);
        let physical = self
            .store
            .upsert_physical_target(&identity, &self.dst.to_string_lossy(), Availability::WillCreate)
            .unwrap();
        physical.id
    }

    fn map_skill(&self, library_id: &str, rel_path: &str, physical_id: &str) -> String {
        let skill_id = format!("{library_id}:{rel_path}");
        let dir_name = rel_path.rsplit('/').next().unwrap_or(rel_path);
        self.store
            .insert_mapping(library_id, &skill_id, physical_id, dir_name)
            .unwrap()
            .id
    }

    fn map_all_valid(&self, library_id: &str, physical_id: &str) -> Vec<String> {
        self.store
            .list_skills(library_id)
            .unwrap()
            .into_iter()
            .filter(|s| s.validation_status == ValidationStatus::Valid && !s.missing)
            .map(|s| {
                self.store
                    .insert_mapping(
                        library_id,
                        &s.id,
                        physical_id,
                        s.rel_path.rsplit('/').next().unwrap_or(&s.rel_path),
                    )
                    .unwrap()
                    .id
            })
            .collect()
    }

    fn plan(&self, library_id: &str, mapping_ids: &[String]) -> SyncPlan {
        planner::create_plan(
            &self.store,
            &CreateSyncPlanInput {
                library_id: library_id.to_string(),
                operation: PlanOperation::Sync,
                mapping_ids: mapping_ids.to_vec(),
            },
        )
        .unwrap()
    }

    fn execute(&self, plan: &SyncPlan) -> String {
        self.execute_op(plan, TaskKind::Sync)
    }

    fn execute_op(&self, plan: &SyncPlan, kind: TaskKind) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        self.store
            .insert_task(&task_id, kind, TaskTrigger::Manual, Some(&plan.library_id), Some(&plan.plan_id))
            .unwrap();
        let sink = NullSink;
        let runner = Runner {
            store: &self.store,
            backups: &self.backups,
            data_dir: &self.data,
            sink: &sink,
        };
        runner
            .run_plan(&plan.plan_id, plan.plan_version, &task_id, TaskTrigger::Manual, Arc::new(AtomicBool::new(false)))
            .unwrap();
        task_id
    }

    fn target_skill_dir(&self, dir_name: &str) -> PathBuf {
        self.dst.join(dir_name)
    }

    fn digest_of(&self, dir: &Path) -> Option<String> {
        scanner::digest_directory(dir).unwrap().map(|(d, _)| d)
    }
}

fn task_view(env: &Env, task_id: &str) -> TaskSnapshotView {
    env.store.task_snapshot_view(task_id).unwrap()
}

fn plan_item<'p>(plan: &'p SyncPlan, skill_name: &str) -> &'p PlanItem {
    plan.groups
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.skill_name == skill_name)
        .unwrap_or_else(|| panic!("计划中应包含 {skill_name}"))
}

// ---------------------------------------------------------------------------
// 主闭环：扫描 → 预览 → 同步 → 校验（AC-04/06/07/08/09）
// ---------------------------------------------------------------------------

#[test]
fn ac04_ac06_first_sync_creates_target_and_baseline() {
    let env = Env::new();
    env.write_skill("code-review", "检查代码", &["脚本内容", "模板内容"]);
    env.write_skill("meeting-notes", "会议纪要", &["参考内容"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);

    let plan = env.plan(&lib, &mappings);
    assert_eq!(plan.summary.create_count, 2);
    assert_eq!(plan.summary.executable_count, 2);
    let item = plan_item(&plan, "code-review");
    assert_eq!(item.action, PlanAction::Create);
    assert!(item
        .file_changes
        .iter()
        .any(|f| f.kind == "add" && f.rel_path == "SKILL.md"));
    assert!(item
        .file_changes
        .iter()
        .any(|f| f.rel_path == "assets/file-0.txt"));

    let task = env.execute(&plan);
    let view = task_view(&env, &task);
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.counts.success, 2);

    // 完整资源按清单复制，摘要一致（AC-04）
    let src_digest = env.digest_of(&env.src.join("code-review")).unwrap();
    let dst_digest = env.digest_of(&env.target_skill_dir("code-review")).unwrap();
    assert_eq!(src_digest, dst_digest);
    assert!(env.target_skill_dir("meeting-notes").join("SKILL.md").exists());

    // 基线登记（AC-06）
    let baseline = env.store.get_baseline(&mappings[0]).unwrap().unwrap();
    assert_eq!(baseline.digest.as_deref(), Some(src_digest.as_str()));

    // 首次创建记录「原先不存在」（§7.4）
    let snapshots = env.store.list_snapshots(Some(&mappings[0])).unwrap();
    assert!(snapshots.iter().any(|s| !s.existed_before));
}

#[test]
fn ac07_repeat_sync_is_noop() {
    let env = Env::new();
    env.write_skill("demo-skill", "演示", &["x"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    let backup_count_before = env.store.list_snapshots(None).unwrap().len();
    let plan2 = env.plan(&lib, &mappings);
    assert_eq!(plan2.summary.executable_count, 0, "重复同步不应有可执行项");
    assert_eq!(plan_item(&plan2, "demo-skill").action, PlanAction::Skip);
    let task2 = env.execute(&plan2);
    let view = task_view(&env, &task2);
    assert_eq!(view.counts.success, 0);
    let backup_count_after = env.store.list_snapshots(None).unwrap().len();
    assert_eq!(backup_count_before, backup_count_after, "无变化不制造多余备份（AC-07）");
}

#[test]
fn ac08_source_update_updates_only_that_skill() {
    let env = Env::new();
    env.write_skill("skill-a", "A", &["v1"]);
    env.write_skill("skill-b", "B", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));
    let b_before = env.digest_of(&env.target_skill_dir("skill-b"));

    // 只改 skill-a
    std::fs::write(env.src.join("skill-a").join("assets/file-0.txt"), "v2-更新").unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    assert_eq!(plan_item(&plan, "skill-a").action, PlanAction::Update);
    assert_eq!(plan_item(&plan, "skill-b").action, PlanAction::Skip);
    env.execute(&plan);

    assert_eq!(
        std::fs::read_to_string(env.target_skill_dir("skill-a").join("assets/file-0.txt")).unwrap(),
        "v2-更新"
    );
    assert_eq!(env.digest_of(&env.target_skill_dir("skill-b")), b_before, "其他技能目录不变（AC-08）");
}

#[test]
fn ac09_managed_file_deletion_propagates_with_backup() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["保留", "将删除"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 源端删除技能内一个文件（§8.1：更新时删除对应目标文件）
    std::fs::remove_file(env.src.join("demo").join("assets/file-1.txt")).unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    let item = plan_item(&plan, "demo");
    assert_eq!(item.action, PlanAction::Update);
    assert!(
        item.file_changes
            .iter()
            .any(|f| f.kind == "delete" && f.rel_path == "assets/file-1.txt"),
        "预览必须列出文件删除（AC-09）"
    );
    env.execute(&plan);
    assert!(!env.target_skill_dir("demo").join("assets/file-1.txt").exists());
    assert!(env.target_skill_dir("demo").join("assets/file-0.txt").exists());

    // 旧文件可从备份恢复
    let snapshots = env.store.list_snapshots(Some(&mappings[0])).unwrap();
    let backup = snapshots.iter().find(|s| s.existed_before).expect("更新前应有备份");
    let backup_dir = env.backups.snapshot_dir(backup.backup_dir.as_deref().unwrap());
    assert!(backup_dir.join("assets/file-1.txt").exists(), "旧文件应在备份中可恢复");
}

// ---------------------------------------------------------------------------
// 冲突（AC-10/11/12/13/16）
// ---------------------------------------------------------------------------

#[test]
fn ac10_unmanaged_same_name_never_silently_overwritten() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["源内容"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mapping = env.map_skill(&lib, "demo", &pt);

    // 目标端预先存在不同内容的非托管目录
    let pre = env.target_skill_dir("demo");
    std::fs::create_dir_all(&pre).unwrap();
    std::fs::write(pre.join("SKILL.md"), "---\nname: demo\ndescription: 旧版本\n---\n").unwrap();
    std::fs::write(pre.join("用户自建.txt"), "用户数据").unwrap();

    let plan = env.plan(&lib, &[mapping.clone()]);
    let item = plan_item(&plan, "demo");
    assert_eq!(item.state_before, MatrixCellState::UnmanagedConflict);
    assert_eq!(item.action, PlanAction::Skip);
    assert!(!item.selected, "非托管冲突默认不纳入（AC-10）");
    assert_eq!(
        item.conflict.as_ref().unwrap().available_choices,
        vec![ConflictChoice::KeepTarget, ConflictChoice::TakeOver]
    );
    env.execute(&plan);
    assert!(pre.join("用户自建.txt").exists(), "非托管内容不得被静默覆盖");

    // 显式接管并用源覆盖：先备份（§6.3/§7.4）
    let plan = env.plan(&lib, &[mapping.clone()]);
    let item = plan_item(&plan, "demo");
    let plan = planner::resolve_conflict(
        &env.store,
        &plan.plan_id,
        plan.plan_version,
        &item.item_id,
        ConflictChoice::TakeOver,
    )
    .unwrap();
    assert_eq!(plan_item(&plan, "demo").action, PlanAction::TakeOver);
    env.execute(&plan);
    assert!(!pre.join("用户自建.txt").exists(), "接管后以源为准");
    assert_eq!(
        env.digest_of(&env.src.join("demo")),
        env.digest_of(&pre),
        "接管后目标与源一致"
    );
    let snapshots = env.store.list_snapshots(Some(&mapping)).unwrap();
    assert!(
        snapshots.iter().any(|s| s.existed_before),
        "接管不同内容的目录必须先备份（§7.4）"
    );
}

#[test]
fn ac11_ac12_target_drift_and_both_modified_are_conflicts() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 目标独自修改 → 目标漂移冲突，不自动覆盖（AC-11）
    std::fs::write(env.target_skill_dir("demo").join("assets/file-0.txt"), "目标本地改动").unwrap();
    let plan = env.plan(&lib, &mappings);
    let item = plan_item(&plan, "demo");
    assert_eq!(item.state_before, MatrixCellState::TargetModified);
    assert_eq!(item.action, PlanAction::Skip);
    env.execute(&plan);
    assert_eq!(
        std::fs::read_to_string(env.target_skill_dir("demo").join("assets/file-0.txt")).unwrap(),
        "目标本地改动",
        "目标漂移默认保留（AC-11）"
    );

    // 双方修改 → both_modified（AC-12）
    std::fs::write(env.src.join("demo").join("assets/file-0.txt"), "源端新改动").unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    assert_eq!(plan_item(&plan, "demo").state_before, MatrixCellState::BothModified);

    // 保留目标后目标不被覆盖
    let item = plan_item(&plan, "demo");
    let plan = planner::resolve_conflict(
        &env.store, &plan.plan_id, plan.plan_version, &item.item_id, ConflictChoice::KeepTarget,
    )
    .unwrap();
    env.execute(&plan);
    assert_eq!(
        std::fs::read_to_string(env.target_skill_dir("demo").join("assets/file-0.txt")).unwrap(),
        "目标本地改动"
    );
}

#[test]
fn ac13_source_removed_is_retained_not_deleted() {
    let env = Env::new();
    env.write_skill("keep-me", "演示", &["内容"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 源端删除整个技能
    std::fs::remove_dir_all(env.src.join("keep-me")).unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    let item = plan_item(&plan, "keep-me");
    assert_eq!(item.state_before, MatrixCellState::SourceRemoved);
    assert_eq!(item.action, PlanAction::Skip);
    env.execute(&plan);
    assert!(
        env.target_skill_dir("keep-me").join("SKILL.md").exists(),
        "源已移除的整项技能不自动删除（AC-13）"
    );
}

#[test]
fn ac16_ownership_conflict_blocked() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["x"]);
    let lib_a = env.register_and_scan();
    let pt = env.add_target();
    env.map_skill(&lib_a, "demo", &pt);

    // 第二个源库抢占同名目标 → 唯一归属约束（AC-16）
    let src_b = env._tmp.path().join("other-lib");
    std::fs::create_dir_all(src_b.join("demo")).unwrap();
    std::fs::write(
        src_b.join("demo").join("SKILL.md"),
        "---\nname: demo\ndescription: 另一个库的同名技能\n---\n",
    )
    .unwrap();
    let lib_b = uuid::Uuid::new_v4().to_string();
    let canonical_b = windows_paths::canonicalize(&src_b).unwrap();
    env.store
        .insert_library(
            &lib_b,
            "库B",
            &canonical_b.to_string_lossy(),
            Some(&canonical_b.to_string_lossy()),
            Some(&windows_paths::fold_case(&canonical_b)),
            "collection",
            None,
        )
        .unwrap();
    let scanned_b = scanner::scan_source_root(
        &canonical_b,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    env.store.apply_scan(&lib_b, &scanned_b).unwrap();
    let err = env
        .store
        .insert_mapping(&lib_b, &format!("{lib_b}:demo"), &pt, "demo")
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ConflictUnresolved, "禁止后台抢占（AC-16）");
}

// ---------------------------------------------------------------------------
// 计划失效与取消（AC-17/21）
// ---------------------------------------------------------------------------

#[test]
fn ac17_stale_plan_rejected_after_content_change() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    let plan = env.plan(&lib, &mappings);

    // 预览后源被编辑（AC-17）
    std::fs::write(env.src.join("demo").join("assets/file-0.txt"), "预览后改动").unwrap();
    let err = planner::validate_for_execute(&env.store, &plan.plan_id, plan.plan_version).unwrap_err();
    assert_eq!(err.code, ErrorCode::PlanStale, "内容变化后旧计划必须拒绝执行");
    assert_eq!(
        env.store.get_plan(&plan.plan_id).unwrap().status,
        PlanStatus::Stale
    );

    // 目标端被编辑同样失效
    let plan2 = env.plan(&lib, &mappings);
    std::fs::create_dir_all(env.target_skill_dir("demo")).unwrap();
    std::fs::write(env.target_skill_dir("demo").join("第三方.txt"), "x").unwrap();
    let err = planner::validate_for_execute(&env.store, &plan2.plan_id, plan2.plan_version).unwrap_err();
    assert_eq!(err.code, ErrorCode::PlanStale);
}

#[test]
fn ac17b_config_change_invalidates_plan() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    let plan = env.plan(&lib, &mappings);
    // 忽略规则变化 → 配置版本递增 → 计划失效（§4.3/§7.2）
    env.store.update_library_ignores(&lib, &["demo/assets/**".to_string()]).unwrap();
    let err = planner::validate_for_execute(&env.store, &plan.plan_id, plan.plan_version).unwrap_err();
    assert_eq!(err.code, ErrorCode::PlanStale);
}

#[test]
fn ac21_cancel_stops_unstarted_items() {
    let env = Env::new();
    env.write_skill("skill-a", "A", &["x"]);
    env.write_skill("skill-b", "B", &["y"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    let plan = env.plan(&lib, &mappings);

    let task_id = uuid::Uuid::new_v4().to_string();
    env.store
        .insert_task(&task_id, TaskKind::Sync, TaskTrigger::Manual, Some(&lib), Some(&plan.plan_id))
        .unwrap();
    let cancel = Arc::new(AtomicBool::new(true)); // 开始前已请求取消
    let sink = NullSink;
    let runner = Runner {
        store: &env.store,
        backups: &env.backups,
        data_dir: &env.data,
        sink: &sink,
    };
    runner
        .run_plan(&plan.plan_id, plan.plan_version, &task_id, TaskTrigger::Manual, cancel)
        .unwrap();
    let view = task_view(&env, &task_id);
    assert_eq!(view.counts.cancelled, 2, "未开始项全部取消（AC-21）");
    assert_eq!(view.counts.success, 0);
    assert!(!env.target_skill_dir("skill-a").exists(), "取消项不写入");
}

// ---------------------------------------------------------------------------
// 备份失败与崩溃恢复（AC-19/20）
// ---------------------------------------------------------------------------

#[test]
fn ac19_backup_failure_blocks_overwrite() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));
    let before = env.digest_of(&env.target_skill_dir("demo"));

    // 源更新
    std::fs::write(env.src.join("demo").join("assets/file-0.txt"), "v2").unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    assert_eq!(plan_item(&plan, "demo").action, PlanAction::Update);

    // 让备份不可用：backups 目录替换为同名文件
    std::fs::remove_dir_all(env.data.join("backups")).unwrap();
    std::fs::write(env.data.join("backups"), "blocked").unwrap();

    let task = env.execute(&plan);
    let view = task_view(&env, &task);
    let item = view.items.iter().find(|i| i.skill_name == "demo").unwrap();
    assert_eq!(item.status, TaskItemStatus::Failed);
    assert_eq!(
        item.error.as_ref().unwrap().code,
        ErrorCode::BackupFailed,
        "备份失败项明确失败（AC-19）"
    );
    assert_eq!(
        env.digest_of(&env.target_skill_dir("demo")),
        before,
        "备份失败不得继续覆盖：目标保持原内容"
    );
}

/// 构造「崩溃现场」：事务目录 + 日志 + DB 事务记录，然后跑启动恢复。
/// new_content_src 用于期望摘要；phase=new_moved_in 时暂存已改名到目标，不再创建 staging。
fn stage_crash(
    env: &Env,
    mapping_id: &str,
    phase: &str,
    target: &Path,
    old_content_src: Option<&Path>,
    new_content_src: Option<&Path>,
) -> String {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let tx_root = skilldock_lib::fsops::sibling_transaction_root(&env.dst).join(&tx_id);
    std::fs::create_dir_all(&tx_root).unwrap();
    let old_dir = tx_root.join("old");
    let staging = tx_root.join("staging");
    if let Some(src) = old_content_src {
        skilldock_lib::fsops::copy_tree(src, &old_dir).unwrap();
    }
    if let Some(src) = new_content_src {
        if phase != "new_moved_in" {
            skilldock_lib::fsops::copy_tree(src, &staging).unwrap();
        }
    }
    env.store
        .upsert_transaction(&TransactionRow {
            id: tx_id.clone(),
            task_item_id: None,
            mapping_id: Some(mapping_id.to_string()),
            physical_target_id: None,
            skill_dir_name: target.file_name().unwrap().to_string_lossy().to_string(),
            phase: phase.to_string(),
            staging_path: Some(staging.to_string_lossy().to_string()),
            old_path: Some(old_dir.to_string_lossy().to_string()),
            target_path: target.to_string_lossy().to_string(),
            backup_id: None,
            expected_source_digest: new_content_src.and_then(|p| env.digest_of(p)),
            expected_old_digest: old_content_src.and_then(|p| env.digest_of(p)),
        })
        .unwrap();
    tx_id
}

#[test]
fn ac20_crash_before_commit_rolls_back_old_version() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 准备 v2 源；制造「旧目录已移出、新目录未就位」的崩溃现场
    env.write_skill("demo", "演示", &["v2"]);
    let target = env.target_skill_dir("demo");
    let v1_copy = env._tmp.path().join("v1-copy");
    skilldock_lib::fsops::copy_tree(&target, &v1_copy).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
    stage_crash(&env, &mappings[0], "old_moved_out", &target, Some(&v1_copy), Some(&env.src.join("demo")));

    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let mut seq = || 1u64;
    let outcome = recovery.startup_recovery(&mut seq).unwrap();
    assert!(outcome.manual.is_empty(), "不应需要人工处理");
    assert!(target.join("assets/file-0.txt").exists(), "重启后恢复旧版本（AC-20）");
    assert_eq!(env.digest_of(&target), env.digest_of(&v1_copy));
    assert_eq!(
        std::fs::read_to_string(target.join("assets/file-0.txt")).unwrap(),
        "v1"
    );
}

#[test]
fn ac20b_crash_after_move_in_commits_success() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 「暂存已改名到目标、基线未提交」现场：目标已是 v2，old 残留
    env.write_skill("demo", "演示", &["v2"]);
    let target = env.target_skill_dir("demo");
    let v1_copy = env._tmp.path().join("v1-copy");
    skilldock_lib::fsops::copy_tree(&target, &v1_copy).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
    skilldock_lib::fsops::copy_tree(&env.src.join("demo"), &target).unwrap();
    stage_crash(&env, &mappings[0], "new_moved_in", &target, Some(&v1_copy), Some(&env.src.join("demo")));

    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let mut seq = || 1u64;
    let outcome = recovery.startup_recovery(&mut seq).unwrap();
    assert!(outcome.manual.is_empty());
    // 继续登记成功：基线更新为 v2（AC-20）
    let baseline = env.store.get_baseline(&mappings[0]).unwrap().unwrap();
    assert_eq!(baseline.digest, env.digest_of(&env.src.join("demo")));
    assert!(
        skilldock_lib::fsops::sibling_transaction_root(&env.dst)
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "事务目录应被清理"
    );
}

#[test]
fn ac20c_unknown_content_preserved_for_manual_recovery() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 目标出现第三方未知内容 + 存在未完成事务 → 不覆盖，转人工
    let target = env.target_skill_dir("demo");
    std::fs::write(target.join("第三方改动.txt"), "未知").unwrap();
    stage_crash(&env, &mappings[0], "old_moved_out", &target, None, None);

    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let mut seq = || 1u64;
    let outcome = recovery.startup_recovery(&mut seq).unwrap();
    assert_eq!(outcome.manual.len(), 1, "未知内容必须保留现场（§8.4.5）");
    assert_eq!(outcome.manual[0].recoverable, "manual");
    assert!(target.join("第三方改动.txt").exists(), "未知内容不得被覆盖");
}

// ---------------------------------------------------------------------------
// 历史恢复（AC-22/23/24）
// ---------------------------------------------------------------------------

#[test]
fn ac22_restore_brings_back_previous_version() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));

    // 更新到 v2
    std::fs::write(env.src.join("demo").join("assets/file-0.txt"), "v2").unwrap();
    env.rescan(&lib);
    let update_task = env.execute(&env.plan(&lib, &mappings));
    assert_eq!(
        std::fs::read_to_string(env.target_skill_dir("demo").join("assets/file-0.txt")).unwrap(),
        "v2"
    );

    // 从历史恢复更新任务 → 回到 v1（AC-22）
    let view = task_view(&env, &update_task);
    let item_id = view.items.iter().find(|i| i.skill_name == "demo").unwrap().item_id.clone();
    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let plan = recovery
        .create_restore_plan(&CreateRestorePlanInput {
            task_id: update_task.clone(),
            item_ids: vec![item_id],
        })
        .unwrap();
    assert_eq!(plan.operation, PlanOperation::Restore);
    let item = plan_item(&plan, "demo");
    assert_eq!(item.action, PlanAction::Restore);
    assert!(item.needs_backup, "恢复前对当前目标再次快照（§6.5）");
    env.execute_op(&plan, TaskKind::Restore);
    assert_eq!(
        std::fs::read_to_string(env.target_skill_dir("demo").join("assets/file-0.txt")).unwrap(),
        "v1",
        "恢复后回到历史版本"
    );
    // 恢复结果登记为新基线；映射暂停自动同步（§6.5）
    let baseline = env.store.get_baseline(&mappings[0]).unwrap().unwrap();
    assert_eq!(baseline.digest, env.digest_of(&env.target_skill_dir("demo")));
    let mapping = env.store.get_mapping(&mappings[0]).unwrap();
    assert_eq!(mapping.paused_reason.as_deref(), Some("restored"));
}

#[test]
fn ac22b_restore_first_create_removes_undrifted_target() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["x"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    let create_task = env.execute(&env.plan(&lib, &mappings));
    assert!(env.target_skill_dir("demo").exists());

    // 撤销首次创建（原先不存在 → 恢复为不存在；AC-22）
    let view = task_view(&env, &create_task);
    let item_id = view.items.iter().find(|i| i.skill_name == "demo").unwrap().item_id.clone();
    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let plan = recovery
        .create_restore_plan(&CreateRestorePlanInput {
            task_id: create_task,
            item_ids: vec![item_id],
        })
        .unwrap();
    let item = plan_item(&plan, "demo");
    assert_eq!(item.action, PlanAction::Restore);
    assert!(item.restore.as_ref().unwrap().to_absent);
    env.execute_op(&plan, TaskKind::Restore);
    assert!(!env.target_skill_dir("demo").exists(), "撤销未漂移的首次创建");
    // 基线登记为「不存在」标记
    let baseline = env.store.get_baseline(&mappings[0]).unwrap().unwrap();
    assert!(baseline.digest.is_none(), "恢复为不存在时记录删除状态（§7.4）");
}

#[test]
fn ac23_restore_with_later_edits_shows_conflict() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    let task1 = env.execute(&env.plan(&lib, &mappings));

    // 恢复点前目标又被编辑（AC-23）
    std::fs::write(env.target_skill_dir("demo").join("用户新增.txt"), "后续修改").unwrap();
    let view = task_view(&env, &task1);
    let item_id = view.items.iter().find(|i| i.skill_name == "demo").unwrap().item_id.clone();
    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let plan = recovery
        .create_restore_plan(&CreateRestorePlanInput {
            task_id: task1,
            item_ids: vec![item_id.clone()],
        })
        .unwrap();
    let item = plan_item(&plan, "demo");
    assert_eq!(
        item.conflict.as_ref().unwrap().kind,
        "restore_drift",
        "恢复前目标又被编辑 → 显示冲突"
    );
    assert!(!item.selected);

    // 未选择不执行：目标保持
    env.execute_op(&plan, TaskKind::Restore);
    assert!(env.target_skill_dir("demo").join("用户新增.txt").exists());

    // 用户明确确认恢复覆盖 → 执行；当前内容先快照
    let plan = recovery
        .create_restore_plan(&CreateRestorePlanInput {
            task_id: view.task_id.clone(),
            item_ids: vec![item_id],
        })
        .unwrap();
    let item = plan_item(&plan, "demo");
    let plan = planner::resolve_conflict(
        &env.store, &plan.plan_id, plan.plan_version, &item.item_id, ConflictChoice::OverwriteWithSource,
    )
    .unwrap();
    assert_eq!(plan_item(&plan, "demo").action, PlanAction::Restore);
    env.execute_op(&plan, TaskKind::Restore);
    assert!(!env.target_skill_dir("demo").join("用户新增.txt").exists(), "确认后覆盖");
    let snapshots = env.store.list_snapshots(Some(&mappings[0])).unwrap();
    assert!(snapshots.len() >= 2, "恢复前保存了当前快照");
}

#[test]
fn ac24_pruned_snapshot_blocks_restore_without_touching_target() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["v1"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));
    std::fs::write(env.src.join("demo").join("assets/file-0.txt"), "v2").unwrap();
    env.rescan(&lib);
    let update_task = env.execute(&env.plan(&lib, &mappings));
    let before = env.digest_of(&env.target_skill_dir("demo"));

    // 把更新前的快照标记清理并删除内容（模拟备份过期/损坏）
    let view = task_view(&env, &update_task);
    let item = view.items.iter().find(|i| i.skill_name == "demo").unwrap();
    let snapshot = env.store.get_snapshot(item.snapshot_id.as_ref().unwrap()).unwrap();
    let dir = env.backups.snapshot_dir(snapshot.backup_dir.as_deref().unwrap());
    std::fs::remove_dir_all(&dir).unwrap();

    let recovery = Recovery { store: &env.store, backups: &env.backups, data_dir: &env.data };
    let plan = recovery
        .create_restore_plan(&CreateRestorePlanInput {
            task_id: update_task,
            item_ids: vec![item.item_id.clone()],
        })
        .unwrap();
    let item = plan_item(&plan, "demo");
    assert_eq!(item.action, PlanAction::Blocked, "备份缺失的项阻塞恢复（AC-24）");
    assert!(item.blocked_reason.as_ref().unwrap().contains("过期"));
    env.execute_op(&plan, TaskKind::Restore);
    assert_eq!(env.digest_of(&env.target_skill_dir("demo")), before, "当前目标不变");
}

// ---------------------------------------------------------------------------
// 共享物理目标与扫描边界（AC-15/02/03/05/26）
// ---------------------------------------------------------------------------

#[test]
fn ac15_shared_physical_target_copied_once() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["x"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    // 两个工具适配器指向同一真实目录 → 同一物理目标
    let pt2 = env
        .store
        .upsert_physical_target(
            &windows_paths::fold_case(&env.dst),
            &env.dst.to_string_lossy(),
            Availability::WillCreate,
        )
        .unwrap();
    assert_eq!(pt, pt2.id, "同一真实目录复用同一物理目标（§5.2.3）");
    let m1 = env.map_skill(&lib, "demo", &pt);
    let plan = env.plan(&lib, &[m1]);
    assert_eq!(plan.groups.len(), 1, "同一物理目标只执行一次（AC-15）");
    env.execute(&plan);
    assert!(env.target_skill_dir("demo").join("SKILL.md").exists());
}

#[test]
fn ac02_ac03_candidate_discovery_and_no_nested_skills() {
    let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
    // 结构 A：根目录即技能集合
    let a = root.join("proj-a");
    std::fs::create_dir_all(a.join("skill-one")).unwrap();
    std::fs::write(a.join("skill-one/SKILL.md"), "---\nname: skill-one\ndescription: d\n---\n").unwrap();
    // 结构 B：技能在 skills/ 子目录
    let b = root.join("proj-b");
    std::fs::create_dir_all(b.join("skills/skill-two")).unwrap();
    std::fs::write(b.join("skills/skill-two/SKILL.md"), "---\nname: skill-two\ndescription: d\n---\n").unwrap();
    std::fs::write(b.join("src.txt"), "code").unwrap();

    let cands_a = scanner::discover_candidates(&a);
    assert_eq!(cands_a.len(), 1);
    assert_eq!(cands_a[0].origin, "root");
    let cands_b = scanner::discover_candidates(&b);
    assert_eq!(cands_b.len(), 1);
    assert_eq!(cands_b[0].origin, "skills");

    // 多候选：根 + skills/ 都有 → 两个候选待选择（AC-02）
    let c = root.join("proj-c");
    std::fs::create_dir_all(c.join("skill-x")).unwrap();
    std::fs::create_dir_all(c.join("skills/skill-y")).unwrap();
    std::fs::write(c.join("skill-x/SKILL.md"), "---\nname: skill-x\ndescription: d\n---\n").unwrap();
    std::fs::write(c.join("skills/skill-y/SKILL.md"), "---\nname: skill-y\ndescription: d\n---\n").unwrap();
    assert_eq!(scanner::discover_candidates(&c).len(), 2);

    // 技能内的示例 SKILL.md 不被当成独立技能（AC-03）
    let s = root.join("lib");
    std::fs::create_dir_all(s.join("real-skill/references/example")).unwrap();
    std::fs::write(s.join("real-skill/SKILL.md"), "---\nname: real-skill\ndescription: d\n---\n").unwrap();
    std::fs::write(
        s.join("real-skill/references/example/SKILL.md"),
        "---\nname: example\ndescription: 嵌套示例\n---\n",
    )
    .unwrap();
    let scanned = scanner::scan_source_root(
        &s,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].dir_name, "real-skill");
}

#[test]
fn ac05_invalid_skills_blocked_but_listed() {
    let env = Env::new();
    env.write_skill("good-skill", "好技能", &[]);
    // 无效 YAML
    let bad = env.src.join("bad-yaml");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(bad.join("SKILL.md"), "---\nname: [unclosed\ndescription: x\n---\n").unwrap();
    // 名称与目录不一致
    let mismatched = env.src.join("dir-name");
    std::fs::create_dir_all(&mismatched).unwrap();
    std::fs::write(
        mismatched.join("SKILL.md"),
        "---\nname: other-name\ndescription: 不一致\n---\n",
    )
    .unwrap();
    // 缺 description
    let nodesc = env.src.join("no-desc");
    std::fs::create_dir_all(&nodesc).unwrap();
    std::fs::write(nodesc.join("SKILL.md"), "---\nname: no-desc\n---\n").unwrap();

    let lib = env.register_and_scan();
    let skills = env.store.list_skills(&lib).unwrap();
    let by_rel = |rel: &str| skills.iter().find(|s| s.rel_path == rel).unwrap();
    assert_eq!(by_rel("good-skill").validation_status, ValidationStatus::Valid);
    let yaml_bad = by_rel("bad-yaml");
    assert_eq!(yaml_bad.validation_status, ValidationStatus::Invalid);
    assert!(yaml_bad.validation.iter().any(|i| i.code == "yaml_error" && i.line.is_some()),
        "YAML 错误需带行号（§4.2）");
    assert_eq!(by_rel("dir-name").validation_status, ValidationStatus::Invalid);
    assert!(by_rel("dir-name").validation.iter().any(|i| i.code == "name_dir_mismatch"));
    assert_eq!(by_rel("no-desc").validation_status, ValidationStatus::Invalid);

    // 无效项在计划中阻塞，不影响正常项（AC-05）
    let pt = env.add_target();
    let good = env.map_skill(&lib, "good-skill", &pt);
    let plan = env.plan(&lib, &[good]);
    env.execute(&plan);
    assert!(env.target_skill_dir("good-skill").join("SKILL.md").exists());
    assert!(!env.target_skill_dir("bad-yaml").exists(), "无效项不同步");
    // 源内容不被改写
    assert!(bad.join("SKILL.md").exists());
}

#[test]
fn ac26_chinese_and_space_paths_work() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("我的 技能库");
    let dst = tmp.path().join("目标 目录/skills");
    std::fs::create_dir_all(src.join("code-review")).unwrap();
    std::fs::write(
        src.join("code-review/SKILL.md"),
        "---\nname: code-review\ndescription: 中文描述 带空格\n---\n\n中文正文\n",
    )
    .unwrap();
    std::fs::write(src.join("code-review/中文 文件.txt"), "中文内容").unwrap();
    let scanned = scanner::scan_source_root(
        &src,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].status, ValidationStatus::Valid);

    let data = tmp.path().join("data");
    let store = Store::open(&data).unwrap();
    let backups = BackupStore::new(&data).unwrap();
    let lib_id = uuid::Uuid::new_v4().to_string();
    let canonical = windows_paths::canonicalize(&src).unwrap();
    store
        .insert_library(
            &lib_id, "中文库", &canonical.to_string_lossy(), Some(&canonical.to_string_lossy()),
            Some(&windows_paths::fold_case(&canonical)), "collection", None,
        )
        .unwrap();
    store.apply_scan(&lib_id, &scanned).unwrap();
    let identity = windows_paths::fold_case(&dst);
    let pt = store
        .upsert_physical_target(&identity, &dst.to_string_lossy(), Availability::WillCreate)
        .unwrap();
    let m = store
        .insert_mapping(&lib_id, &format!("{lib_id}:code-review"), &pt.id, "code-review")
        .unwrap();
    let plan = planner::create_plan(
        &store,
        &CreateSyncPlanInput {
            library_id: lib_id.clone(),
            operation: PlanOperation::Sync,
            mapping_ids: vec![m.id],
        },
    )
    .unwrap();
    let task_id = uuid::Uuid::new_v4().to_string();
    store
        .insert_task(&task_id, TaskKind::Sync, TaskTrigger::Manual, Some(&lib_id), Some(&plan.plan_id))
        .unwrap();
    let sink = NullSink;
    let runner = Runner { store: &store, backups: &backups, data_dir: &data, sink: &sink };
    runner
        .run_plan(&plan.plan_id, plan.plan_version, &task_id, TaskTrigger::Manual, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert!(dst.join("code-review/中文 文件.txt").exists(), "中文与空格路径可用（AC-26）");
}

#[test]
fn ac01_empty_library_is_fine() {
    let env = Env::new();
    let lib = env.register_and_scan();
    let skills = env.store.list_skills(&lib).unwrap();
    assert!(skills.is_empty(), "空技能库：无技能、不报错（AC-01）");
    assert!(!env.dst.exists() || env.dst.read_dir().unwrap().next().is_none());
}

#[test]
fn ac14_disable_mapping_keeps_disk_content() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["x"]);
    let lib = env.register_and_scan();
    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));
    env.store.set_mapping_enabled(&mappings[0], false).unwrap();
    assert!(env.target_skill_dir("demo").join("SKILL.md").exists(), "停止管理不删除磁盘文件（AC-14）");
}

#[test]
fn ignore_patterns_exclude_and_shrink_managed_files() {
    let env = Env::new();
    env.write_skill("demo", "演示", &["a", "b"]);
    // 加一个小文件
    std::fs::write(env.src.join("demo/tmp-note.txt"), "临时").unwrap();
    let lib = env.register_and_scan();
    env.store.update_library_ignores(&lib, &["demo/tmp-note.txt".to_string()]).unwrap();
    env.rescan(&lib);
    let skills = env.store.list_skills(&lib).unwrap();
    let demo = skills.iter().find(|s| s.rel_path == "demo").unwrap();
    assert!(demo.excluded.contains(&"tmp-note.txt".to_string()), "忽略规则生效");
    assert!(!demo.manifest.iter().any(|f| f.rel_path == "tmp-note.txt"));

    let pt = env.add_target();
    let mappings = env.map_all_valid(&lib, &pt);
    env.execute(&env.plan(&lib, &mappings));
    assert!(!env.target_skill_dir("demo").join("tmp-note.txt").exists(), "被忽略文件不分发");

    // 忽略规则变化后，托管文件因此被移除时列为文件删除（§4.3）
    std::fs::write(env.src.join("demo/assets/now-ignored.txt"), "x").unwrap();
    env.rescan(&lib);
    env.execute(&env.plan(&lib, &mappings));
    assert!(env.target_skill_dir("demo").join("assets/now-ignored.txt").exists());
    env.store
        .update_library_ignores(&lib, &["demo/tmp-note.txt".to_string(), "demo/assets/now-ignored.txt".to_string()])
        .unwrap();
    env.rescan(&lib);
    let plan = env.plan(&lib, &mappings);
    let item = plan_item(&plan, "demo");
    assert!(
        item.file_changes.iter().any(|f| f.kind == "delete" && f.rel_path == "assets/now-ignored.txt"),
        "忽略导致的托管移除必须列为文件删除（§4.3）"
    );
    assert!(!item.excluded_by_ignore.is_empty(), "预览显示被排除文件");
}
