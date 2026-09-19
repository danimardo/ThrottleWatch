pub mod access;
mod commands;
mod config;
pub mod diagnostics;
mod export;
pub mod ipc;
pub mod logging;
pub mod ports;
pub mod release_manifest;
pub mod storage;
pub mod telemetry;
mod test_support;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args().any(|argument| argument == "--elevated-launcher") {
        let _ = ipc::elevated::run_elevated_launcher();
        return;
    }
    let _dev_config = config::DevConfig::load();
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            std::fs::create_dir_all(&data_dir)?;
            let log_guard = logging::init(data_dir.join("logs"), false)?;
            let database = data_dir.join("throttlewatch.db");
            let storage = storage::Storage::open(database)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(log_guard);
            app.manage(storage::AppState::new(storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_live_snapshot,
            commands::get_coverage,
            commands::recheck_coverage,
            commands::request_low_level_access,
            commands::disable_advanced_access
        ])
        .run(tauri::generate_context!())
        .expect("error while running ThrottleWatch");
}
