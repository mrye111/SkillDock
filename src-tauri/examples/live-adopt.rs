//! 运维工具：数据库丢失后的一键恢复接管（§11.2 / AC-27 场景）。
//!
//! 用途：重新登记技能库与预设目标 → 扫描 → 对「目标内容与源逐字节一致」的项
//! 批量建立托管基线（不写任何文件）。内容不同的项保持冲突待用户在界面处理。
//!
//! 用法（在已安装应用的机器上）：
//!   cargo run --example live-adopt -- "D:\code\SkillDock\skills"
//! 环境变量：SKILLDOCK_DATA_DIR 覆盖数据目录（默认 %LOCALAPPDATA%\com.skilldock.app）。

use skilldock_lib::backup::BackupStore;
use skilldock_lib::contract::*;
use skilldock_lib::executor::{NullSink, Runner};
use skilldock_lib::planner;
use skilldock_lib::scanner::{self, ScanOptions};
use skilldock_lib::storage::Store;
use skilldock_lib::windows_paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() {
    if let Err(e) = run() {
        eprintln!("失败: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let lib_path = std::env::args()
        .nth(1)
        .expect("用法: live-adopt <技能库目录>");
    let data_dir = std::env::var_os("SKILLDOCK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("com.skilldock.app")
        });
    println!("数据目录: {}", data_dir.display());
    let store = Store::open(&data_dir)?;
    let backups = BackupStore::new(&data_dir)?;

    // 1. 登记（或复用）技能库
    let canonical = windows_paths::canonicalize(Path::new(&lib_path))?;
    let identity = windows_paths::fold_case(&canonical);
    let library_id = match store.find_library_by_identity(&identity)? {
        Some(l) => {
            println!("复用既有库: {} ({})", l.display_name, l.id);
            l.id
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            let name = canonical
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "技能库".into());
            store.insert_library(
                &id,
                &name,
                &canonical.to_string_lossy(),
                Some(&canonical.to_string_lossy()),
                Some(&identity),
                "collection",
                None,
            )?;
            println!("登记新库: {name} ({id})");
            id
        }
    };

    // 2. 扫描
    let scanned = scanner::scan_source_root(
        &canonical,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )?;
    let (skills, _) = store.apply_scan(&library_id, &scanned)?;
    let valid: Vec<_> = skills
        .iter()
        .filter(|s| s.validation_status == ValidationStatus::Valid && !s.missing)
        .collect();
    println!("扫描: {} 技能，{} 有效", skills.len(), valid.len());

    // 3. 三个用户级预设目标（Codex 实测路径 / Claude Code / Cursor）
    let home = std::env::var("USERPROFILE")?;
    let target_dirs = [
        format!(r"{home}\.codex\skills"),
        format!(r"{home}\.claude\skills"),
        format!(r"{home}\.cursor\skills"),
    ];
    let mut mapping_ids: Vec<String> = Vec::new();
    for dir in &target_dirs {
        let canon = windows_paths::canonicalize(Path::new(dir))?;
        let pt = store.upsert_physical_target(
            &windows_paths::fold_case(&canon),
            &canon.to_string_lossy(),
            Availability::Exists,
        )?;
        for skill in &valid {
            let dir_name = skill
                .rel_path
                .rsplit('/')
                .next()
                .unwrap_or(&skill.rel_path);
            match store.insert_mapping(&library_id, &skill.id, &pt.id, dir_name) {
                Ok(m) => mapping_ids.push(m.id),
                Err(e) => println!("  跳过映射 {} -> {}: {}", skill.rel_path, dir, e.message),
            }
        }
    }
    println!("映射总数: {}", mapping_ids.len());

    // 4. 计划
    let plan = planner::create_plan(
        &store,
        &CreateSyncPlanInput {
            library_id: library_id.clone(),
            operation: PlanOperation::Sync,
            mapping_ids: mapping_ids.clone(),
        },
    )?;
    println!(
        "计划: 新增 {} / 更新 {} / 冲突 {} / 跳过 {} / 阻塞 {}",
        plan.summary.create_count,
        plan.summary.update_count,
        plan.summary.conflict_count,
        plan.summary.skip_count,
        plan.summary.blocked_count
    );

    // 5. 批量接管「内容一致」项（只建基线，不写文件）
    let (plan, outcome) = planner::resolve_conflicts_bulk(
        &store,
        &plan.plan_id,
        plan.plan_version,
        &["same_content".to_string()],
        ConflictChoice::AdoptExisting,
    )?;
    println!("批量接管: {} 项（跳过 {} 项）", outcome.applied.len(), outcome.skipped.len());

    // 6. 执行（adopt 不产生任何文件写入）
    let task_id = uuid::Uuid::new_v4().to_string();
    store.insert_task(
        &task_id,
        TaskKind::Sync,
        TaskTrigger::Manual,
        Some(&library_id),
        Some(&plan.plan_id),
    )?;
    let sink = NullSink;
    let runner = Runner { store: &store, backups: &backups, data_dir: &data_dir, sink: &sink };
    runner.run_plan(
        &plan.plan_id,
        plan.plan_version,
        &task_id,
        TaskTrigger::Manual,
        Arc::new(AtomicBool::new(false)),
    )?;
    let view = store.task_snapshot_view(&task_id)?;
    println!(
        "执行完成: 成功 {} / 失败 {} / 跳过 {} / 冲突待处理 {}",
        view.counts.success, view.counts.failed, view.counts.skipped, view.counts.conflict_pending
    );
    println!("其余「待新增/待更新/冲突」项请在应用界面预览后决定。");
    Ok(())
}
