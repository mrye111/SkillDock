//! SkillDock 后端库入口：模块注册 + Tauri 应用装配。

pub mod adapters;
pub mod backup;
pub mod commands;
pub mod contract;
pub mod error;
pub mod executor;
pub mod fsops;
pub mod planner;
pub mod recovery;
pub mod scanner;
pub mod state;
pub mod storage;
pub mod windows_paths;

use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 单实例（FR-10）：二次启动时聚焦既有窗口
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            // 数据目录：默认 %LOCALAPPDATA%/SkillDock（§11.2）；测试/便携可用环境变量覆盖
            let data_dir = std::env::var_os("SKILLDOCK_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    app.path()
                        .app_local_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                });
            let state = Arc::new(state::AppState::new(&data_dir)?);

            // 启动先扫描未完成事务（§8.4.6）
            let recovery = recovery::Recovery {
                store: &state.store,
                backups: &state.backups,
                data_dir: &state.data_dir,
            };
            let seq = Arc::clone(&state.seq);
            let mut next = move || {
                seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
            };
            match recovery.startup_recovery(&mut next) {
                Ok(outcome) => {
                    use tauri::Emitter;
                    for manual in outcome.manual {
                        if let Err(e) =
                            app.emit(contract::EVENT_RECOVERY_REQUIRED, &manual)
                        {
                            eprintln!("[skilldock] 恢复事件发送失败: {e}");
                        }
                    }
                    if !outcome.auto_recovered.is_empty() {
                        eprintln!(
                            "[skilldock] 启动恢复完成：{} 项",
                            outcome.auto_recovered.len()
                        );
                    }
                }
                Err(e) => eprintln!("[skilldock] 启动恢复失败: {e}"),
            }

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::register_library,
            commands::select_library_root,
            commands::list_libraries,
            commands::set_library_pinned,
            commands::remove_library,
            commands::update_library_settings,
            commands::scan_library,
            commands::get_library,
            commands::list_adapters,
            commands::save_target,
            commands::list_targets,
            commands::remove_target,
            commands::update_mappings,
            commands::create_sync_plan,
            commands::resolve_conflict,
            commands::execute_sync_plan,
            commands::cancel_task,
            commands::get_task_snapshot,
            commands::list_tasks,
            commands::list_history,
            commands::get_task_detail,
            commands::create_restore_plan,
            commands::get_backup_stats,
            commands::list_snapshots,
            commands::update_backup_settings,
            commands::open_registered_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SkillDock");
}
