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

use std::sync::Mutex;
use tauri::Manager;

/// Keeps the collector runtime alive for the life of the application; stopped on exit.
struct CollectorHandle(Mutex<telemetry::runtime::CollectorRuntime>);

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
            app.manage(commands::GuidedController::default());

            // The collector runs for the whole life of the application: nothing on screen is real
            // until this delivers samples.
            let live = commands::LiveHandle::default();
            app.manage(live.clone());
            let rules = diagnostics::Ruleset::v1()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let config = telemetry::runtime::RuntimeConfig::from_ruleset(&rules)
                .ok_or_else(|| std::io::Error::other("ruleset lacks the collector parameters"))?;
            let observer =
                std::sync::Arc::new(commands::TauriObserver { app: app.handle().clone() });
            let runtime = telemetry::runtime::CollectorRuntime::start(
                telemetry::launch::collector_launcher(),
                live.0,
                observer,
                config,
            )?;
            app.manage(CollectorHandle(Mutex::new(runtime)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_live_snapshot,
            commands::get_coverage,
            commands::recheck_coverage,
            commands::get_onboarding_state,
            commands::set_onboarding_state,
            commands::get_window_state,
            commands::set_window_state,
            commands::request_low_level_access,
            commands::disable_advanced_access,
            commands::get_guided_preflight,
            commands::start_guided,
            commands::stop_guided,
            commands::get_analysis_window,
            commands::get_cpu_topology
        ])
        .build(tauri::generate_context!())
        .expect("error while building ThrottleWatch")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(collector) = app.try_state::<CollectorHandle>()
                && let Ok(mut runtime) = collector.0.lock()
            {
                runtime.stop();
            }
        });
}
