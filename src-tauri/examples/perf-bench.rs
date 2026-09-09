//! §13 性能基准：200 技能 / 5,000 文件 / 100 MiB 合成库。
//!
//! 用法：cargo run --release --example perf-bench
//! 输出各阶段耗时，对照 §13 预算：扫描 ≤5s；同步 3 目标 ≤30s。

use skilldock_lib::backup::BackupStore;
use skilldock_lib::contract::*;
use skilldock_lib::executor::{EventSink, Runner};
use skilldock_lib::planner;
use skilldock_lib::scanner::{self, ScanOptions};
use skilldock_lib::storage::Store;
use skilldock_lib::windows_paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 记录阶段边界事件时间戳的 sink（用于剖析执行器各阶段耗时）
#[derive(Default)]
struct ProfileSink {
    events: Mutex<Vec<(String, Instant)>>,
    seq: AtomicU64,
}
impl EventSink for ProfileSink {
    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        if event == EVENT_SYNC_PROGRESS {
            let phase = payload
                .get("phase")
                .and_then(|p| p.as_str())
                .unwrap_or("?")
                .to_string();
            self.events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((phase, Instant::now()));
        }
    }
    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }
}

const SKILLS: usize = 200;
const FILES_PER_SKILL: usize = 25;
const FILE_BYTES: usize = 20_000; // ≈ 100 MiB 总量

fn main() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let src = root.join("src-lib");
    let data = root.join("appdata");
    let dsts: Vec<PathBuf> = (0..3).map(|i| root.join(format!("target-{i}/skills"))).collect();

    println!("== 生成合成库：{SKILLS} 技能 × {FILES_PER_SKILL} 文件 × {FILE_BYTES}B ==");
    let t = Instant::now();
    gen_library(&src);
    let total_bytes = dir_bytes(&src);
    println!(
        "生成完毕：{:.1} MiB，耗时 {:?}",
        total_bytes as f64 / 1024.0 / 1024.0,
        t.elapsed()
    );

    let store = Store::open(&data).unwrap();
    let backups = BackupStore::new(&data).unwrap();
    let canonical = windows_paths::canonicalize(&src).unwrap();
    let lib_id = uuid::Uuid::new_v4().to_string();
    store
        .insert_library(
            &lib_id,
            "基准库",
            &canonical.to_string_lossy(),
            Some(&canonical.to_string_lossy()),
            Some(&windows_paths::fold_case(&canonical)),
            "collection",
            None,
        )
        .unwrap();

    // ---- 1. 扫描（预算 ≤5s）----
    let t = Instant::now();
    let scanned = scanner::scan_source_root(
        &canonical,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    let scan_time = t.elapsed();
    let t = Instant::now();
    store.apply_scan(&lib_id, &scanned).unwrap();
    let apply_time = t.elapsed();
    println!(
        "[1] 扫描+摘要: {:?}（预算 ≤5s，{}）| 落库: {:?}",
        scan_time,
        if scan_time.as_secs_f64() <= 5.0 { "PASS" } else { "FAIL" },
        apply_time
    );

    // ---- 2. 目标与映射 ----
    let mut mapping_ids = Vec::new();
    let skills = store.list_skills(&lib_id).unwrap();
    for dst in &dsts {
        let pt = store
            .upsert_physical_target(
                &windows_paths::fold_case(dst),
                &dst.to_string_lossy(),
                Availability::WillCreate,
            )
            .unwrap();
        for s in &skills {
            mapping_ids.push(
                store
                    .insert_mapping(
                        &lib_id,
                        &s.id,
                        &pt.id,
                        s.rel_path.rsplit('/').next().unwrap(),
                    )
                    .unwrap()
                    .id,
            );
        }
    }

    // ---- 3. 首次同步（100 MiB × 3 目标，预算 ≤30s）----
    let plan = planner::create_plan(
        &store,
        &CreateSyncPlanInput {
            library_id: lib_id.clone(),
            operation: PlanOperation::Sync,
            mapping_ids: mapping_ids.clone(),
        },
    )
    .unwrap();
    println!(
        "[3] 生成计划（{} 项）: 见下",
        plan.groups.iter().map(|g| g.items.len()).sum::<usize>()
    );
    let task_id = uuid::Uuid::new_v4().to_string();
    store
        .insert_task(&task_id, TaskKind::Sync, TaskTrigger::Manual, Some(&lib_id), Some(&plan.plan_id))
        .unwrap();
    let sink = ProfileSink::default();
    let runner = Runner { store: &store, backups: &backups, data_dir: &data, sink: &sink };
    let t = Instant::now();
    runner
        .run_plan(&plan.plan_id, plan.plan_version, &task_id, TaskTrigger::Manual, Arc::new(AtomicBool::new(false)))
        .unwrap();
    let sync_time = t.elapsed();
    println!(
        "[4] 首次同步 3 目标: {:?}（预算 ≤30s，{}）",
        sync_time,
        if sync_time.as_secs_f64() <= 30.0 { "PASS" } else { "FAIL" }
    );
    // 阶段耗时剖析：相邻事件的时间差
    {
        let events = sink.events.lock().unwrap_or_else(|e| e.into_inner());
        let mut totals: std::collections::HashMap<String, std::time::Duration> =
            std::collections::HashMap::new();
        for w in events.windows(2) {
            let seg = format!("{} → {}", w[0].0, w[1].0);
            *totals.entry(seg).or_default() += w[1].1 - w[0].1;
        }
        let mut v: Vec<_> = totals.into_iter().collect();
        v.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        for (seg, d) in v.iter().take(8) {
            println!("    {seg}: {:.1?}", d);
        }
    }

    // ---- 5. 矩阵计算（get_library 核心成本）----
    let librow = store.get_library(&lib_id).unwrap();
    let skills = store.list_skills(&lib_id).unwrap();
    let t = Instant::now();
    let _cells = planner::matrix_cells(&store, &librow, &skills).unwrap();
    let matrix_first = t.elapsed();
    let t = Instant::now();
    let _cells = planner::matrix_cells(&store, &librow, &skills).unwrap();
    let matrix_second = t.elapsed();
    println!("[5] 矩阵（600 单元格）: 首次 {:?} | 再次 {:?}", matrix_first, matrix_second);

    // ---- 6. 增量更新（10 文件变更 → 计划 + 复核 + 执行）----
    for i in 0..10usize {
        let f = src.join(format!("skill-{i:03}/assets/f0.dat"));
        std::fs::write(&f, format!("changed-content-{i}")).unwrap();
    }
    let scanned = scanner::scan_source_root(
        &canonical,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    store.apply_scan(&lib_id, &scanned).unwrap();
    let t = Instant::now();
    let plan2 = planner::create_plan(
        &store,
        &CreateSyncPlanInput {
            library_id: lib_id.clone(),
            operation: PlanOperation::Sync,
            mapping_ids: mapping_ids.clone(),
        },
    )
    .unwrap();
    let plan_time = t.elapsed();
    let updates = plan2.groups.iter().flat_map(|g| g.items.iter()).filter(|i| i.selected).count();
    let t = Instant::now();
    let _ = planner::validate_for_execute(&store, &plan2.plan_id, plan2.plan_version).unwrap();
    let validate_time = t.elapsed();
    println!(
        "[6] 增量计划（{updates} 项选中）: 生成 {:?} | 执行前复核 {:?}",
        plan_time, validate_time
    );

    println!("== 基准结束 ==");
}

fn gen_library(src: &Path) {
    let payload = "x".repeat(FILE_BYTES);
    for i in 0..SKILLS {
        let dir = src.join(format!("skill-{i:03}"));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: skill-{i:03}\ndescription: 基准技能 {i}\n---\n\n# {i}\n"),
        )
        .unwrap();
        for j in 0..FILES_PER_SKILL - 1 {
            std::fs::write(dir.join(format!("assets/f{j}.dat")), &payload).unwrap();
        }
    }
}

fn dir_bytes(dir: &Path) -> u64 {
    walkdir_size(dir)
}

fn walkdir_size(dir: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                total += walkdir_size(&p);
            } else if let Ok(m) = p.metadata() {
                total += m.len();
            }
        }
    }
    total
}
