//! Tauri Commands（契约 §4）。薄封装：参数校验 → 核心服务 → 错误映射。
//! 执行类命令只接受登记对象 ID 与计划 ID；实际路径由后端解析（§10.1/§10.4）。

use crate::adapters;
use crate::contract::*;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::executor::{EventSink, Runner};
use crate::planner;
use crate::recovery::Recovery;
use crate::scanner::{self, ScanOptions};
use crate::state::{AppState, TauriSink};
use crate::storage::{Store, TaskItemRow};
use crate::windows_paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

// ---------------------------------------------------------------------------
// 4.1 技能库
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn register_library(state: St, input: RegisterLibraryInput) -> AppResult<RegisterLibraryResult> {
    let raw = PathBuf::from(&input.path);
    let mut diagnostics = Vec::new();
    if let Some(reason) = windows_paths::unsupported_location_reason(&raw) {
        diagnostics.push(PathDiagnostic {
            level: "error".into(),
            code: "unsupported_location".into(),
            message: reason,
        });
        return Err(AppError::new(
            ErrorCode::Unsupported,
            format!("不支持的位置：{}", input.path),
        ));
    }
    if !raw.is_dir() {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("目录不存在或不可读：{}", input.path),
        ));
    }
    let canonical = windows_paths::canonicalize(&raw)?;
    let identity = windows_paths::fold_case(&canonical);

    // 相同真实目录只登记一次（§7.1）
    if let Some(existing) = state.store.find_library_by_identity(&identity)? {
        state.store.touch_library(&existing.id)?;
        return build_register_result(&state.store, &existing.id, diagnostics);
    }

    let mode = input.mode.as_deref().unwrap_or("auto");
    let (scan_mode, source_root, skill_filter, _needs_choice, candidates) = if mode == "single" {
        // 添加单个技能：目录本身即一个 Skill（§4.1）
        if scanner::discover_candidates(canonical.parent().unwrap_or(&canonical))
            .is_empty()
            && !canonical.join("SKILL.md").exists()
        {
            return Err(AppError::new(
                ErrorCode::ValidationFailed,
                "该目录不包含 SKILL.md，不是一个技能文件夹",
            ));
        }
        let parent = canonical
            .parent()
            .ok_or_else(|| AppError::invalid_path("技能目录没有父目录"))?
            .to_path_buf();
        let dir_name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| AppError::invalid_path("技能目录名无效"))?;
        (
            "single",
            Some(parent.to_string_lossy().to_string()),
            Some(vec![dir_name]),
            false,
            Vec::new(),
        )
    } else {
        let candidates = scanner::discover_candidates(&canonical);
        let chosen = input.selected_root.clone().map(PathBuf::from).or_else(|| {
            if candidates.len() == 1 {
                Some(PathBuf::from(&candidates[0].path))
            } else {
                None
            }
        });
        let root = chosen
            .map(|p| windows_paths::canonicalize(&p).unwrap_or(p))
            .map(|p| p.to_string_lossy().to_string());
        (
            "collection",
            root,
            None,
            candidates.len() > 1 && input.selected_root.is_none(),
            candidates,
        )
    };
    let display_name = input.display_name.clone().unwrap_or_else(|| {
        canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "技能库".into())
    });
    let library_id = uuid::Uuid::new_v4().to_string();
    state.store.insert_library(
        &library_id,
        &display_name,
        &canonical.to_string_lossy(),
        source_root.as_deref(),
        Some(&identity),
        scan_mode,
        skill_filter.as_deref(),
    )?;
    let _ = &candidates;
    build_register_result(&state.store, &library_id, diagnostics)
}

fn build_register_result(
    store: &Store,
    library_id: &str,
    diagnostics: Vec<PathDiagnostic>,
) -> AppResult<RegisterLibraryResult> {
    let lib = store.get_library(library_id)?;
    let candidates = if lib.scan_mode == "single" {
        Vec::new()
    } else {
        scanner::discover_candidates(Path::new(&lib.canonical_path))
    };
    Ok(RegisterLibraryResult {
        library_id: lib.id.clone(),
        canonical_path: lib.canonical_path.clone(),
        source_root: lib.source_root.clone(),
        needs_root_choice: lib.source_root.is_none() && candidates.len() > 1,
        candidates,
        diagnostics,
        recent_libraries: list_library_summaries(store)?,
    })
}

fn list_library_summaries(store: &Store) -> AppResult<Vec<LibrarySummary>> {
    let libs = store.list_libraries(false)?;
    libs.into_iter()
        .take(10)
        .map(|l| library_summary(store, l))
        .collect()
}

fn library_summary(store: &Store, l: crate::storage::LibraryRow) -> AppResult<LibrarySummary> {
    let skills = store.list_skills(&l.id).unwrap_or_default();
    Ok(LibrarySummary {
        library_id: l.id,
        display_name: l.display_name,
        canonical_path: l.canonical_path.clone(),
        source_root: l.source_root,
        pinned: l.pinned,
        last_opened_at: l.last_opened_at,
        path_valid: Path::new(&l.canonical_path).is_dir(),
        skill_count: skills.iter().filter(|s| !s.missing).count() as u32,
        invalid_count: skills
            .iter()
            .filter(|s| s.validation_status != ValidationStatus::Valid && !s.missing)
            .count() as u32,
    })
}

#[tauri::command]
pub fn select_library_root(
    state: St,
    library_id: String,
    root: String,
) -> AppResult<RegisterLibraryResult> {
    let lib = state.store.get_library(&library_id)?;
    let canon_root = windows_paths::canonicalize(Path::new(&root))?;
    let valid = scanner::discover_candidates(Path::new(&lib.canonical_path))
        .iter()
        .any(|c| {
            windows_paths::fold_case(Path::new(&c.path))
                == windows_paths::fold_case(&canon_root)
        });
    if !valid {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "所选源根不在该库的候选位置中",
        ));
    }
    state
        .store
        .set_library_source_root(&library_id, &canon_root.to_string_lossy())?;
    state.store.touch_library(&library_id)?;
    build_register_result(&state.store, &library_id, Vec::new())
}

#[tauri::command]
pub fn list_libraries(state: St) -> AppResult<Vec<LibrarySummary>> {
    list_library_summaries(&state.store)
}

#[tauri::command]
pub fn set_library_pinned(state: St, library_id: String, pinned: bool) -> AppResult<()> {
    state.store.set_library_pinned(&library_id, pinned)
}

#[tauri::command]
pub fn remove_library(state: St, library_id: String) -> AppResult<()> {
    state.store.archive_library(&library_id)
}

#[tauri::command]
pub fn update_library_settings(
    state: St,
    library_id: String,
    ignore_patterns: Vec<String>,
) -> AppResult<serde_json::Value> {
    // 先验证 glob 合法性再落库
    scanner::build_globset(&ignore_patterns)?;
    let version = state
        .store
        .update_library_ignores(&library_id, &ignore_patterns)?;
    Ok(serde_json::json!({ "configVersion": version }))
}

#[tauri::command]
pub fn scan_library(
    state: St,
    app: tauri::AppHandle,
    library_id: String,
) -> AppResult<serde_json::Value> {
    let lib = state.store.get_library(&library_id)?;
    let task_id = uuid::Uuid::new_v4().to_string();
    state.store.insert_task(
        &task_id,
        TaskKind::Scan,
        TaskTrigger::Manual,
        Some(&library_id),
        None,
    )?;
    let app_state = state.inner().clone();
    let sink = TauriSink {
        app,
        seq: state.seq.clone(),
    };
    let tid = task_id.clone();
    std::thread::spawn(move || {
        let _ = scan_task_body(app_state, sink, tid, lib);
    });
    Ok(serde_json::json!({ "taskId": task_id }))
}

fn scan_task_body(
    state: Arc<AppState>,
    sink: TauriSink,
    task_id: String,
    lib: crate::storage::LibraryRow,
) -> AppResult<()> {
    state.store.update_task_status(&task_id, TaskStatus::Running)?;
    let started = std::time::Instant::now();
    let result = (|| -> AppResult<(Vec<crate::storage::SkillRow>, usize)> {
        let source_root = lib.source_root.clone().ok_or_else(|| {
            AppError::new(ErrorCode::ValidationFailed, "请先选定源根目录")
        })?;
        let ignore = scanner::build_globset(&lib.ignore_patterns)?;
        let scanned_dirs = AtomicU64::new(0);
        let tid = task_id.clone();
        let on_dir = |p: &Path| {
            let n = scanned_dirs.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            sink.emit_json(
                EVENT_SCAN_PROGRESS,
                serde_json::to_value(ScanProgressEvent {
                    task_id: tid.clone(),
                    seq: sink.next_seq(),
                    scanned_dirs: n as u32,
                    discovered_skills: n as u32,
                    invalid_count: 0,
                    current_path: Some(p.to_string_lossy().to_string()),
                    done: false,
                })
                .unwrap_or_default(),
            );
        };
        let scanned = scanner::scan_source_root(
            Path::new(&source_root),
            &ScanOptions {
                ignore: Some(&ignore),
                skill_filter: lib.skill_filter.as_deref(),
                on_dir: Some(&on_dir),
            },
        )?;
        let count = scanned.len();
        let (rows, _newly_missing) = state.store.apply_scan(&lib.id, &scanned)?;
        Ok((rows, count))
    })();
    match result {
        Ok((rows, scanned_count)) => {
            let invalid = rows
                .iter()
                .filter(|s| s.validation_status != ValidationStatus::Valid && !s.missing)
                .count() as u32;
            let valid = rows
                .iter()
                .filter(|s| s.validation_status == ValidationStatus::Valid && !s.missing)
                .count() as u32;
            sink.emit_json(
                EVENT_SCAN_PROGRESS,
                serde_json::to_value(ScanProgressEvent {
                    task_id: task_id.clone(),
                    seq: sink.next_seq(),
                    scanned_dirs: scanned_count as u32,
                    discovered_skills: rows.iter().filter(|s| !s.missing).count() as u32,
                    invalid_count: invalid,
                    current_path: None,
                    done: true,
                })
                .unwrap_or_default(),
            );
            let counts = TaskCounts {
                total: rows.len() as u32,
                success: valid,
                failed: 0,
                skipped: 0,
                conflict_pending: invalid,
                cancelled: 0,
            };
            state.store.set_task_counts(&task_id, &counts)?;
            state.store.update_task_status(&task_id, TaskStatus::Completed)?;
            sink.emit_json(
                EVENT_SYNC_COMPLETED,
                serde_json::to_value(SyncCompletedEvent {
                    task_id: task_id.clone(),
                    seq: sink.next_seq(),
                    status: TaskStatus::Completed,
                    counts,
                    duration_ms: started.elapsed().as_millis() as u64,
                })
                .unwrap_or_default(),
            );
            Ok(())
        }
        Err(e) => {
            state.store.set_task_error(&task_id, Some(&e))?;
            state.store.update_task_status(&task_id, TaskStatus::Failed)?;
            sink.emit_json(
                EVENT_SYNC_COMPLETED,
                serde_json::to_value(SyncCompletedEvent {
                    task_id: task_id.clone(),
                    seq: sink.next_seq(),
                    status: TaskStatus::Failed,
                    counts: TaskCounts::default(),
                    duration_ms: started.elapsed().as_millis() as u64,
                })
                .unwrap_or_default(),
            );
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_library(state: St, library_id: String) -> AppResult<LibraryDetail> {
    let lib = state.store.get_library(&library_id)?;
    state.store.touch_library(&library_id)?;
    let skills = state.store.list_skills(&library_id)?;
    let mut cells = planner::matrix_cells(&state.store, &lib, &skills)?;
    let summary = library_summary(&state.store, lib.clone())?;
    let views = skills
        .into_iter()
        .map(|s| SkillView {
            skill_id: s.id.clone(),
            rel_path: s.rel_path.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            file_count: s.file_count,
            total_bytes: s.total_bytes,
            last_content_changed_at: s.last_content_changed_at.clone(),
            digest: s.digest.clone(),
            validation: SkillValidation {
                status: s.validation_status,
                issues: s.validation.clone(),
            },
            targets: cells.remove(&s.id).unwrap_or_default(),
        })
        .collect();
    Ok(LibraryDetail {
        summary,
        ignore_patterns: lib.ignore_patterns,
        config_version: lib.config_version,
        skills: views,
    })
}

// ---------------------------------------------------------------------------
// 4.2 目标管理
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_adapters(state: St, project_root: Option<String>) -> AppResult<Vec<AdapterInfo>> {
    let _ = &state;
    let root = project_root.map(PathBuf::from);
    Ok(adapters::ADAPTERS
        .iter()
        .map(|a| adapters::adapter_info(a, root.as_deref()))
        .collect())
}

#[tauri::command]
pub fn save_target(state: St, input: SaveTargetInput) -> AppResult<SaveTargetResult> {
    let def = adapters::find_adapter(&input.adapter_id)?;
    if !def.scopes.contains(&input.scope) {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            format!("{} 不支持作用域 {:?}", def.display_name, input.scope),
        ));
    }
    if def.id == "custom" && input.display_name.as_deref().unwrap_or("").is_empty() {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "自定义目标需要填写显示名称",
        ));
    }
    let project_root = input.project_root.as_deref().map(PathBuf::from);
    let (raw_path, template, source) = adapters::resolve_adapter_path(
        def,
        input.scope,
        project_root.as_deref(),
        input.path.as_deref(),
    )?;
    let canonical = windows_paths::canonicalize(&raw_path)?;
    let (availability, detail) = adapters::check_availability(&canonical);
    match availability {
        Availability::UnsupportedLocation => {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                detail.unwrap_or_else(|| "不支持的位置".into()),
            ))
        }
        Availability::InvalidPath => {
            return Err(AppError::invalid_path(
                detail.unwrap_or_else(|| "路径无效".into()),
            ))
        }
        _ => {}
    }

    // §5.2.5：源根与目标根相同/互为父子、多目标根互相包含 → 阻止
    for lib in state.store.list_libraries(false)? {
        if let Some(root) = &lib.source_root {
            let root = PathBuf::from(root);
            match windows_paths::overlap(&canonical, &root) {
                windows_paths::Overlap::Disjoint => {}
                _ => {
                    return Err(AppError::new(
                        ErrorCode::PathOverlap,
                        format!(
                            "目标目录与技能库 {} 的源根 {} 相同或互相包含，会造成循环/覆盖",
                            lib.display_name, root.display()
                        ),
                    ))
                }
            }
        }
    }
    for t in state.store.list_targets()? {
        let existing = state.store.get_physical_target(&t.physical_target_id)?;
        let existing_path = PathBuf::from(&existing.canonical_path);
        if windows_paths::overlap(&canonical, &existing_path)
            != windows_paths::Overlap::Disjoint
            && windows_paths::fold_case(&canonical) != windows_paths::fold_case(&existing_path)
        {
            return Err(AppError::new(
                ErrorCode::PathOverlap,
                format!(
                    "目标目录与既有目标 {} 互相包含，会造成循环/覆盖",
                    existing.canonical_path
                ),
            ));
        }
    }

    let identity = windows_paths::fold_case(&canonical);
    let physical = state.store.upsert_physical_target(
        &identity,
        &canonical.to_string_lossy(),
        availability,
    )?;
    let target_id = uuid::Uuid::new_v4().to_string();
    let display_name = input
        .display_name
        .clone()
        .unwrap_or_else(|| def.display_name.to_string());
    state.store.insert_target(
        &target_id,
        def.id,
        adapters::ADAPTER_VERSION,
        input.scope,
        &display_name,
        &template,
        &source,
        input.project_root.as_deref(),
        &physical.id,
    )?;
    let shared: Vec<String> = state
        .store
        .targets_on_physical(&physical.id)?
        .into_iter()
        .filter(|t| t.id != target_id)
        .map(|t| t.display_name)
        .collect();
    let mut warnings = adapters::shared_read_warnings(def.id, &canonical);
    if !shared.is_empty() {
        warnings.push(format!("该目录已与 {} 共用一个物理目标", shared.join("、")));
    }
    Ok(SaveTargetResult {
        target_id,
        physical_target_id: physical.id,
        resolved_path: canonical.to_string_lossy().to_string(),
        resolution_source: source,
        availability,
        shared_with_tools: shared,
        warnings,
    })
}

#[tauri::command]
pub fn list_targets(state: St) -> AppResult<Vec<TargetView>> {
    let targets = state.store.list_targets()?;
    targets
        .into_iter()
        .map(|t| {
            let physical = state.store.get_physical_target(&t.physical_target_id)?;
            let shared: Vec<String> = state
                .store
                .targets_on_physical(&physical.id)?
                .into_iter()
                .filter(|x| x.id != t.id)
                .map(|x| x.display_name)
                .collect();
            let mapped_count = state.store.count_mappings_on_physical(&physical.id)?;
            Ok(TargetView {
                target_id: t.id,
                physical_target_id: physical.id.clone(),
                adapter_id: t.adapter_id,
                display_name: t.display_name,
                scope: t.scope,
                resolved_path: physical.canonical_path.clone(),
                resolution_source: t.resolution_source,
                availability: physical.availability,
                shared_with_tools: shared,
                mapped_skill_count: mapped_count,
                enabled: t.enabled,
            })
        })
        .collect()
}

#[tauri::command]
pub fn remove_target(state: St, target_id: String) -> AppResult<()> {
    state.store.disable_target(&target_id)
}

#[tauri::command]
pub fn update_mappings(
    state: St,
    library_id: String,
    skill_id: String,
    physical_target_ids: Vec<String>,
) -> AppResult<serde_json::Value> {
    let skill = state.store.get_skill(&skill_id)?;
    if skill.library_id != library_id {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "技能不属于该技能库",
        ));
    }
    if skill.validation_status != ValidationStatus::Valid {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "校验失败的技能不能设置目标",
        ));
    }
    let dir_name = skill
        .rel_path
        .rsplit('/')
        .next()
        .unwrap_or(&skill.rel_path)
        .to_string();
    let existing = state.store.mappings_for_skill(&skill_id)?;
    for pt in &physical_target_ids {
        state.store.get_physical_target(pt)?; // 校验存在
        let active = existing
            .iter()
            .find(|m| &m.physical_target_id == pt && m.enabled);
        if active.is_none() {
            // 同名既有（已停用）映射复用：先检查再新建
            let revived = existing
                .iter()
                .find(|m| &m.physical_target_id == pt && !m.enabled);
            if let Some(m) = revived {
                state.store.set_mapping_enabled(&m.id, true)?;
            } else {
                state
                    .store
                    .insert_mapping(&library_id, &skill_id, pt, &dir_name)?;
            }
        }
    }
    for m in existing.iter().filter(|m| m.enabled) {
        if !physical_target_ids.contains(&m.physical_target_id) {
            // 取消勾选：仅停止映射，不删除磁盘内容（AC-14）
            state.store.set_mapping_enabled(&m.id, false)?;
        }
    }
    let mappings = state
        .store
        .mappings_for_skill(&skill_id)?
        .into_iter()
        .filter(|m| m.enabled)
        .map(|m| MappingView {
            mapping_id: m.id,
            skill_id: m.skill_id,
            physical_target_id: m.physical_target_id,
            target_dir_name: m.target_dir_name,
            enabled: m.enabled,
            paused_reason: m.paused_reason,
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({ "mappings": mappings }))
}

// ---------------------------------------------------------------------------
// 4.3 计划与执行
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_sync_plan(state: St, input: CreateSyncPlanInput) -> AppResult<SyncPlan> {
    planner::create_plan(&state.store, &input)
}

#[tauri::command]
pub fn resolve_conflict(
    state: St,
    plan_id: String,
    plan_version: i64,
    item_id: String,
    choice: ConflictChoice,
) -> AppResult<SyncPlan> {
    planner::resolve_conflict(&state.store, &plan_id, plan_version, &item_id, choice)
}

#[tauri::command]
pub fn execute_sync_plan(
    state: St,
    app: tauri::AppHandle,
    plan_id: String,
    plan_version: i64,
) -> AppResult<serde_json::Value> {
    let plan = state.store.get_plan(&plan_id)?;
    if plan.status != PlanStatus::Active {
        return Err(AppError::new(
            ErrorCode::PlanStale,
            "计划已失效或已执行，请重新生成预览",
        ));
    }
    let kind = match plan.operation {
        PlanOperation::Sync => TaskKind::Sync,
        PlanOperation::Remove => TaskKind::Remove,
        PlanOperation::Restore => TaskKind::Restore,
    };
    let task_id = uuid::Uuid::new_v4().to_string();
    state.store.insert_task(
        &task_id,
        kind,
        TaskTrigger::Manual,
        Some(&plan.library_id),
        Some(&plan_id),
    )?;
    let app_state = state.inner().clone();
    let cancel = app_state.register_cancel(&task_id);
    let sink = TauriSink {
        app,
        seq: state.seq.clone(),
    };
    let tid = task_id.clone();
    std::thread::spawn(move || {
        let runner = Runner {
            store: &app_state.store,
            backups: &app_state.backups,
            data_dir: &app_state.data_dir,
            sink: &sink,
        };
        let _ = runner.run_plan(&plan_id, plan_version, &tid, TaskTrigger::Manual, cancel);
        app_state.clear_cancel(&tid);
    });
    Ok(serde_json::json!({ "taskId": task_id }))
}

#[tauri::command]
pub fn cancel_task(state: St, task_id: String) -> AppResult<serde_json::Value> {
    let ok = state.request_cancel(&task_id);
    Ok(serde_json::json!({
        "taskId": task_id,
        "state": if ok { "requested" } else { "not_cancellable" },
    }))
}

#[tauri::command]
pub fn get_task_snapshot(state: St, task_id: String) -> AppResult<TaskSnapshotView> {
    state.store.task_snapshot_view(&task_id)
}

#[tauri::command]
pub fn list_tasks(state: St, active_only: bool) -> AppResult<Vec<TaskSnapshotView>> {
    let tasks = state.store.list_tasks(active_only)?;
    tasks
        .into_iter()
        .map(|t| state.store.task_snapshot_view(&t.id))
        .collect()
}

// ---------------------------------------------------------------------------
// 4.4 历史与恢复
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_history(state: St, query: HistoryQuery) -> AppResult<HistoryPage> {
    let limit = query.limit.unwrap_or(50).min(200);
    let rows = state.store.list_history(
        query.cursor.as_deref(),
        limit + 1,
        query.library_id.as_deref(),
        query.status,
    )?;
    let has_more = rows.len() as u32 > limit;
    let mut rows = rows;
    rows.truncate(limit as usize);
    let next_cursor = if has_more {
        rows.last().map(|r| r.created_at.clone())
    } else {
        None
    };
    let mut items = Vec::new();
    for t in rows {
        let lib_name = t
            .library_id
            .as_deref()
            .and_then(|id| state.store.get_library(id).ok())
            .map(|l| l.display_name);
        let task_items = state.store.list_task_items(&t.id)?;
        let mut target_paths: Vec<String> = task_items
            .iter()
            .map(|i: &TaskItemRow| {
                Path::new(&i.target_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| i.target_path.clone())
            })
            .collect();
        target_paths.sort();
        target_paths.dedup();
        let duration_ms = match (&t.started_at, &t.finished_at) {
            (Some(s), Some(f)) => Some(
                (crate::storage::parse_dt_pub(f) - crate::storage::parse_dt_pub(s))
                    .num_milliseconds()
                    .max(0) as u64,
            ),
            _ => None,
        };
        items.push(HistoryTaskSummary {
            task_id: t.id,
            kind: t.kind,
            trigger: t.trigger,
            started_at: t.started_at,
            duration_ms,
            status: t.status,
            counts: t.counts,
            library_name: lib_name,
            target_paths,
        });
    }
    Ok(HistoryPage { items, next_cursor })
}

#[tauri::command]
pub fn get_task_detail(state: St, task_id: String) -> AppResult<TaskSnapshotView> {
    state.store.task_snapshot_view(&task_id)
}

#[tauri::command]
pub fn create_restore_plan(
    state: St,
    input: CreateRestorePlanInput,
) -> AppResult<SyncPlan> {
    let recovery = Recovery {
        store: &state.store,
        backups: &state.backups,
        data_dir: &state.data_dir,
    };
    recovery.create_restore_plan(&input)
}

#[tauri::command]
pub fn get_backup_stats(state: St) -> AppResult<BackupStats> {
    state.backups.stats(&state.store)
}

#[tauri::command]
pub fn list_snapshots(state: St, mapping_id: String) -> AppResult<Vec<SnapshotView>> {
    let rows = state.store.list_snapshots(Some(&mapping_id))?;
    rows.into_iter()
        .map(|s| {
            let available = state.backups.verify(&s)?;
            Ok(SnapshotView {
                snapshot_id: s.id,
                task_item_id: s.task_item_id,
                mapping_id: s.mapping_id,
                target_path: s.target_path,
                existed_before: s.existed_before,
                digest: s.digest,
                bytes: s.bytes,
                pinned: s.pinned,
                available,
                created_at: s.created_at,
            })
        })
        .collect()
}

#[tauri::command]
pub fn update_backup_settings(
    state: St,
    retention_days: Option<u32>,
    soft_cap_bytes: Option<u64>,
) -> AppResult<BackupStats> {
    if let Some(d) = retention_days {
        state.store.set_setting("backup_retention_days", &d.to_string())?;
    }
    if let Some(b) = soft_cap_bytes {
        state
            .store
            .set_setting("backup_soft_cap_bytes", &b.to_string())?;
    }
    state.backups.stats(&state.store)
}

// ---------------------------------------------------------------------------
// 4.5 受信任位置打开
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_registered_path(state: St, kind: String, id: String) -> AppResult<()> {
    let path = match kind.as_str() {
        "library" => state.store.get_library(&id)?.canonical_path,
        "target" => state.store.get_physical_target(&id)?.canonical_path,
        "task_item" => state.store.get_task_item(&id)?.target_path,
        "snapshot" => {
            let s = state.store.get_snapshot(&id)?;
            match &s.backup_dir {
                Some(dir) => state
                    .backups
                    .snapshot_dir(dir)
                    .to_string_lossy()
                    .to_string(),
                None => s.target_path,
            }
        }
        _ => {
            return Err(AppError::new(
                ErrorCode::ValidationFailed,
                format!("未知的位置类型：{kind}"),
            ))
        }
    };
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("路径当前不存在：{path}"),
        ));
    }
    // 经 Windows 文件管理器打开已验证路径，不拼接 Shell 命令（§7.1）
    std::process::Command::new("explorer")
        .arg(&p)
        .spawn()
        .map_err(AppError::from)?;
    Ok(())
}
