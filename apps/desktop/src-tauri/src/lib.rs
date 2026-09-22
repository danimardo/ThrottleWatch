pub mod access;
pub mod alerting;
mod commands;
mod config;
pub mod diagnostics;
pub mod export;
pub mod i18n;
pub mod ipc;
pub mod lifecycle;
pub mod logging;
pub mod ports;
pub mod preferences;
pub mod release_manifest;
pub mod sampling_control;
pub mod startup;
pub mod storage;
pub mod telemetry;
mod test_support;
pub mod tray;
pub mod updater;
pub mod updates;
pub mod updates_app;

use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Keeps the collector runtime alive for the life of the application; stopped on exit.
pub(crate) struct CollectorHandle(pub(crate) Mutex<telemetry::runtime::CollectorRuntime>);

/// Port Playwright's CDP client attaches to in the `e2e` build (T-PLAY-002). `tauri.conf.json`
/// marks the main window `"create": false` so it is only ever built here, where wry lets us pass
/// `additional_browser_args`; setting `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` instead has no effect
/// because wry always calls `SetAdditionalBrowserArguments` itself, which overrides that env var.
#[cfg(feature = "e2e")]
const E2E_CDP_PORT: &str = "9222";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args().any(|argument| argument == "--elevated-launcher") {
        let _ = ipc::elevated::run_elevated_launcher();
        return;
    }
    let _dev_config = config::DevConfig::load();
    tauri::Builder::default()
        // Must be the first plugin: a second launch focuses the running window instead of
        // opening another process (and another collector).
        .plugin(tauri_plugin_single_instance::init(|app, _arguments, _working_dir| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&'static str>>,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_updater::Builder::new().pubkey(updates_app::plugin_public_key()).build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(false)
                if window.is_minimized().unwrap_or(false)
                    || window.is_visible().is_ok_and(|visible| !visible) =>
            {
                commands::cancel_guided_for_lifecycle(
                    window.app_handle(),
                    diagnostics::guided::GuidedStopReason::WindowHidden,
                );
            }
            // `window.close()` emits this first, whether it was asked for by the custom
            // TitleBar button or by a native close (Alt+F4, the window manager) — one gate
            // for both, for whichever of T090 or T182 applies. `prevent_close` keeps the window
            // open for everything except `Proceed`, where the default close behaves exactly as
            // before this gate existed.
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle();
                match lifecycle::window_close_intent(app) {
                    lifecycle::CloseIntent::Blocked(reason) => {
                        api.prevent_close();
                        let _ = window.emit(
                            "lifecycle:close-blocked",
                            serde_json::json!({ "reason": reason }),
                        );
                    }
                    lifecycle::CloseIntent::DecisionRequired => {
                        api.prevent_close();
                        let _ = window.emit(lifecycle::DECISION_REQUIRED_EVENT, ());
                    }
                    lifecycle::CloseIntent::Hide => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    lifecycle::CloseIntent::Proceed => {}
                }
            }
            tauri::WindowEvent::Destroyed => {
                commands::cancel_guided_for_lifecycle(
                    window.app_handle(),
                    diagnostics::guided::GuidedStopReason::WindowHidden,
                );
            }
            _ => {}
        })
        .setup(|app| {
            let window_config = app.config().app.windows.first().cloned().ok_or_else(|| {
                std::io::Error::other("tauri.conf.json must declare the main window")
            })?;
            let window_builder = tauri::WebviewWindowBuilder::from_config(app, &window_config)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            #[cfg(feature = "e2e")]
            let window_builder = window_builder
                .additional_browser_args(&format!("--remote-debugging-port={E2E_CDP_PORT}"));
            window_builder.build().map_err(|error| std::io::Error::other(error.to_string()))?;

            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            std::fs::create_dir_all(&data_dir)?;
            let database = data_dir.join("throttlewatch.db");
            let (mut storage, recovered) = storage::Storage::open_recovering_corruption(
                database,
                &jiff::Timestamp::now().to_string(),
            )
            .map_err(|error| std::io::Error::other(error.to_string()))?;
            if recovered.is_some() {
                // FR-075: no destructive action on the existing data — it moved aside (next to
                // the new one, `throttlewatch.db.corrupt-<fecha>`), not gone — and the app starts
                // on a fresh database instead of failing outright. The path is not logged: it is
                // exactly where the person already knows to look, in their own data directory.
                tracing::warn!(
                    component = "core",
                    code = "STORAGE_DATABASE_CORRUPT_RECOVERED",
                    msg = "the database could not be opened and was moved aside; a new one was created"
                );
            }
            app.manage(commands::CorruptBackupNotice(std::sync::Mutex::new(recovered)));
            let mut preferences = storage
                .user_preferences()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            // FR-086: detailed logging never survives a process restart, even if its stored
            // expiry timestamp has not elapsed yet.
            if preferences.get("logging.detailed_until").is_some_and(|value| !value.is_null()) {
                preferences.insert("logging.detailed_until".to_owned(), serde_json::Value::Null);
                storage
                    .set_user_preferences(&preferences)
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
            }
            storage
                .abort_orphaned_sessions()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let now = jiff::Timestamp::now().to_string();
            let retention = preferences
                .get("history.retention")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("7d");
            storage
                .prune_sessions(retention, &now)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            if retention == "session" {
                let _ = logging::clear_directory(data_dir.join("logs"));
            }
            let log_guard = logging::init(data_dir.join("logs"), false)?;
            let logging_control = log_guard.control();
            app.manage(log_guard);
            app.manage(logging_control);
            app.manage(commands::FrontendLogLimiter::default());
            app.manage(storage::AppState::new(storage));
            app.manage(commands::GuidedController::default());
            app.manage(commands::TrayController::default());
            app.manage(commands::RecorderHandle::new());
            {
                // FR-075: retry cadence for storage writes, read once at startup from
                // `ruleset-v1` — never a literal. `Ruleset::v1()` is cheap (an embedded JSON
                // parse); the collector's own copy loads a moment later, at line ~209.
                let retry_s = diagnostics::Ruleset::v1()
                    .ok()
                    .and_then(|rules| rules.parameter("storage.retry_s"))
                    .unwrap_or(60.0);
                app.manage(commands::StorageResilience::new((retry_s * 1_000.0) as u64));
            }
            app.manage(commands::ExportController::default());
            app.manage(alerting::AlertHandle::new());
            app.manage(updates_app::BusyOperations::default());
            {
                let updates_enabled = preferences
                    .get("updates.enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let stored = app
                    .state::<storage::AppState>()
                    .storage
                    .lock()
                    .ok()
                    .and_then(|storage| storage.updater_state().ok())
                    .unwrap_or_default();
                let machine = updates_app::restore_machine(stored, updates_enabled);
                let transport = updates_app::PluginTransport::production(
                    app.handle().clone(),
                    Box::new({
                        let handle = app.handle().clone();
                        move || shutdown_services(&handle)
                    }),
                )
                .ok_or_else(|| std::io::Error::other("the update endpoint is not a valid URL"))?;
                app.manage(updates_app::UpdateHandle::new(updates::UpdateService::new(
                    machine,
                    Box::new(transport),
                    env!("CARGO_PKG_VERSION"),
                )));
                updates_app::spawn_scheduler(app.handle().clone());
            }
            tray::setup(app)?;
            startup::apply_launch_mode(app.handle(), &preferences);

            // The collector runs for the whole life of the application: nothing on screen is real
            // until this delivers samples.
            let live = commands::LiveHandle::default();
            app.manage(live.clone());
            let rules = diagnostics::Ruleset::v1()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let config = telemetry::runtime::RuntimeConfig::from_ruleset(&rules)
                .ok_or_else(|| std::io::Error::other("ruleset lacks the collector parameters"))?;
            // The first session already starts at the interval the preferences and the power
            // source ask for (a paused-on-battery plan has none: the runtime starts paused).
            let initial_plan = sampling_control::plan_from_preferences(
                &rules,
                &preferences,
                sampling_control::on_ac_power(),
            );
            if let Some(interval) = initial_plan.and_then(|plan| plan.interval) {
                config
                    .interval_ms
                    .store(interval.as_millis() as u64, std::sync::atomic::Ordering::SeqCst);
            }
            let observer =
                std::sync::Arc::new(commands::TauriObserver { app: app.handle().clone() });
            let runtime = telemetry::runtime::CollectorRuntime::start(
                telemetry::launch::collector_launcher(),
                live.0,
                observer,
                config,
            )?;
            if initial_plan.is_some_and(|plan| plan.interval.is_none()) {
                runtime.set_battery_paused(true);
            }
            app.manage(CollectorHandle(Mutex::new(runtime)));
            sampling_control::spawn_power_watch(app.handle().clone());
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
            commands::preview_export,
            commands::export,
            commands::cancel_export,
            commands::import_session,
            commands::get_preferences,
            commands::get_storage_usage,
            commands::export_corrupt_backup,
            commands::get_technical_summary,
            commands::get_third_party_notices,
            commands::log_frontend,
            commands::delete_monitoring_data,
            commands::reset_application,
            commands::open_logs_folder,
            commands::open_external_url,
            commands::set_preference,
            commands::get_cpu_topology,
            commands::set_tray_paused,
            commands::list_sessions,
            commands::get_session,
            commands::get_report,
            commands::reevaluate_report,
            commands::delete_session,
            commands::set_session_reference,
            commands::confirm_close,
            commands::resolve_first_close,
            updates_app::get_update_state,
            updates_app::check_for_update,
            updates_app::download_update,
            updates_app::install_update
        ])
        .build(tauri::generate_context!())
        .expect("error while building ThrottleWatch")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                shutdown_services(app);
            }
        });
}

/// Stops the collector, closes and freezes the session in progress and applies the retention.
/// Runs on exit and, because the Windows installer ends the process without an exit event, right
/// before an update is installed. Safe to run twice.
pub(crate) fn shutdown_services(app: &tauri::AppHandle) {
    let Some(collector) = app.try_state::<CollectorHandle>() else { return };
    let Ok(mut runtime) = collector.0.lock() else { return };
    runtime.stop();
    drop(runtime);
    if let Some(recorder) = app.try_state::<commands::RecorderHandle>() {
        recorder.flush(app);
    }
    if let Some(state) = app.try_state::<storage::AppState>()
        && let Ok(mut storage) = state.storage.lock()
        && let Ok(preferences) = storage.user_preferences()
    {
        let retention = preferences
            .get("history.retention")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("7d");
        if retention == "session"
            && let Ok(logs) = app.path().app_data_dir().map(|path| path.join("logs"))
        {
            let _ = logging::clear_directory(logs);
        }
        let _ = storage.prune_sessions(retention, &jiff::Timestamp::now().to_string());
    }
}
