use std::sync::Arc;
use tokio::sync::Mutex;

mod config;
mod commands;
mod planner;
mod dag;
mod worker;
mod aggregator;
mod history;
mod session;

use commands::AppState;
use config::AppConfig;
use session::SessionManager;
use worker::pool::WorkerPool;

/// Search for config.toml in multiple locations
fn find_config_path() -> std::path::PathBuf {
    // 1. Next to the executable (for production)
    if let Ok(exe) = std::env::current_exe() {
        let beside_exe = exe.parent().unwrap_or(std::path::Path::new(".")).join("config.toml");
        if beside_exe.exists() {
            return beside_exe;
        }
    }
    // 2. Current working directory
    let cwd = std::env::current_dir().unwrap_or_default().join("config.toml");
    if cwd.exists() {
        return cwd;
    }
    // 3. Project root (src-tauri's parent) — useful during `cargo tauri dev`
    if let Ok(exe) = std::env::current_exe() {
        if let Some(src_tauri) = exe.ancestors().find(|p| p.ends_with("src-tauri") || p.join("src-tauri").exists()) {
            let project_root = if src_tauri.ends_with("src-tauri") {
                src_tauri.parent().unwrap_or(src_tauri)
            } else {
                src_tauri
            };
            let at_root = project_root.join("config.toml");
            if at_root.exists() {
                return at_root;
            }
        }
    }
    // 4. Fallback: CARGO_MANIFEST_DIR at compile time
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let at_manifest_parent = manifest.parent().unwrap_or(&manifest).join("config.toml");
    if at_manifest_parent.exists() {
        return at_manifest_parent;
    }
    // Last resort: cwd
    cwd
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Try multiple locations for config.toml
    let config_path = find_config_path();

    let config = if config_path.exists() {
        eprintln!("Loading config from: {}", config_path.display());
        AppConfig::load(&config_path).unwrap_or_else(|e| {
            eprintln!("Failed to load config: {e}, using defaults");
            AppConfig::default_config()
        })
    } else {
        eprintln!("No config.toml found, using defaults");
        AppConfig::default_config()
    };

    let pool = WorkerPool::new(&config.executor_workers()
        .into_iter().cloned().collect::<Vec<_>>());
    let state = Arc::new(Mutex::new(AppState { config, pool, sessions: SessionManager::new() }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::submit_task,
            commands::get_config,
            commands::get_workers,
            commands::save_config,
            commands::test_worker,
            commands::get_history_list,
            commands::get_history_entry,
            commands::delete_history_entry,
            commands::start_planning,
            commands::send_planner_message,
            commands::approve_plan,
            commands::send_task_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
