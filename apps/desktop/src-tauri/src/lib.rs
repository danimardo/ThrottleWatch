pub mod access;
pub mod alerting;
pub mod appearance;
mod commands;
mod config;
pub mod dev_faults;
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

/// The elevated launcher connection kept alive across collector restarts within this application
/// session (ADR-0004 amendment, 2026-09-28) — deliberately a *separate* managed state from
/// [`CollectorHandle`], so `restart_collector` tearing down and rebuilding the `CollectorRuntime`
/// never touches it. `None` until the first elevated launch of this run.
pub(crate) struct ElevatedConnectionHandle(pub(crate) telemetry::launch::SharedElevatedConnection);

/// Port Playwright's CDP client attaches to in the `e2e` build (T-PLAY-002). `tauri.conf.json`
/// marks the main window `"create": false` so it is only ever built here, where wry lets us pass
/// `additional_browser_args`; setting `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` instead has no effect
/// because wry always calls `SetAdditionalBrowserArguments` itself, which overrides that env var.
#[cfg(feature = "e2e")]
const E2E_CDP_PORT: &str = "9222";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (base_level, level_override_rejected) =
        logging::base_level(logging::BuildKind::current(), config::DevConfig::load().log_level);
    if std::env::args().any(|argument| argument == "--elevated-launcher") {
        // Its own file (XVII): the launcher's typical failure is not reaching the application.
        // Without a log directory it still runs; there is simply nowhere to write.
        let _log = ipc::elevated::launcher_log_directory()
            .and_then(|directory| logging::init_launcher(directory, base_level).ok());
        // Every step and the final result are logged inside; nothing is left to report here.
        let _ = ipc::elevated::run_elevated_launcher();
        return;
    }
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
        .setup(move |app| {
            let window_config = app.config().app.windows.first().cloned().ok_or_else(|| {
                std::io::Error::other("tauri.conf.json must declare the main window")
            })?;
            let window_builder = tauri::WebviewWindowBuilder::from_config(app, &window_config)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            #[cfg(feature = "e2e")]
            let window_builder = window_builder
                .additional_browser_args(&format!("--remote-debugging-port={E2E_CDP_PORT}"));
            window_builder.build().map_err(|error| std::io::Error::other(error.to_string()))?;

            // T181: a native E2E suite drives this very binary, so it must not read or write the
            // real `%APPDATA%\com.throttlewatch.desktop` — it would inherit the developer's
            // onboarding, history and preferences (making the run unreproducible) and risk their
            // data. `TW_DEV_DATA_DIR` moves the whole directory; it is compiled out of release.
            let data_dir = dev_faults::resolve_data_dir(app.handle())
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            std::fs::create_dir_all(&data_dir)?;
            let database = data_dir.join("throttlewatch.db");
            // The log only opens once the preferences say whether to clear it (retention
            // «solo esta sesión»), so what happens before is held here and written right after.
            let mut before_log: Vec<(&'static str, &'static str)> = Vec::new();
            // E2E-13: `TW_DEV_CORRUPT_DB=1` damages the existing database on purpose so the
            // recovery below is the real one, not a simulation. No-op outside debug/`e2e`.
            if dev_faults::corrupt_database_if_requested(&database) {
                before_log.push((
                    "STORAGE_CORRUPTION_FAULT_INJECTED",
                    "TW_DEV_CORRUPT_DB damaged the database before opening it",
                ));
            }
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
                before_log.push((
                    "STORAGE_DATABASE_CORRUPT_RECOVERED",
                    "the database could not be opened and was moved aside; a new one was created",
                ));
            }
            app.manage(commands::CorruptBackupNotice(std::sync::Mutex::new(recovered)));
            // T181: `TW_DEV_SEED_BUNDLE` leaves a session with history without minutes of recording
            // or a native dialog, through the same importer the person's own "Import session" uses.
            // Always logged either way: a seed that silently did not land would make a scenario pass
            // or fail for reasons that have nothing to do with what it tests. No-op outside debug/`e2e`.
            match dev_faults::import_seed_if_requested(&storage) {
                Some(Ok(_)) => before_log.push((
                    "STORAGE_TEST_SEED_IMPORTED",
                    "TW_DEV_SEED_BUNDLE imported a session into the database",
                )),
                Some(Err(error)) => {
                    before_log.push((error.code(), "TW_DEV_SEED_BUNDLE could not be imported"))
                }
                None => {}
            }
            let mut preferences = storage
                .user_preferences()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            // FR-086: detailed logging never survives a process restart, even if its stored
            // expiry timestamp has not elapsed yet — except in a debug build (`cargo build`,
            // no `--release`), which is always a development machine: restoring it here saves
            // re-enabling it by hand on every single launch (spec.md amendment, 2026-09-28).
            let stored_detailed_until = preferences
                .get("logging.detailed_until")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            if !cfg!(debug_assertions) && stored_detailed_until.is_some() {
                preferences.insert("logging.detailed_until".to_owned(), serde_json::Value::Null);
                storage
                    .set_user_preferences(&preferences)
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
            }
            let orphaned = storage
                .abort_orphaned_sessions()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let now = jiff::Timestamp::now().to_string();
            let retention = preferences
                .get("history.retention")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("7d");
            let pruned = storage
                .prune_sessions(retention, &now)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let capped = storage
                .enforce_session_cap(storage::Storage::MAX_RETAINED_SESSIONS)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let schema_version = storage.schema_version().unwrap_or(-1);
            if retention == "session" {
                let _ = logging::clear_directory(data_dir.join("logs"));
            }
            let log_guard = logging::init(data_dir.join("logs"), base_level)?;
            if cfg!(debug_assertions)
                && let Some(until) = stored_detailed_until
            {
                log_guard.control().set_detailed_until(Some(until));
            }
            log_info!(
                "APP_STARTED",
                "ThrottleWatch started",
                version = env!("CARGO_PKG_VERSION"),
                build = logging::BuildKind::current().as_str()
            );
            log_info!(
                "LOG_LEVEL_EFFECTIVE",
                "effective log level at startup",
                level = log_guard.control().effective_level().as_str(),
                build = logging::BuildKind::current().as_str(),
                detailed = log_guard.control().is_detailed()
            );
            if level_override_rejected {
                log_warn!(
                    "LOG_LEVEL_OVERRIDE_INVALID",
                    "TW_DEV_LOG_LEVEL is not one of error|warn|info|debug|trace; the default applies"
                );
            }
            for (code, message) in before_log {
                log_warn!(code, message);
            }
            log_info!(
                component: "storage",
                "STORAGE_OPENED",
                "database opened and retention applied",
                version = schema_version,
                count = orphaned as u64,
                dropped = pruned + capped,
                reason = retention
            );
            if release_manifest::built_for_testing_only() {
                // T112: an optimised build that trusts the development key. It can start a
                // locally signed collector, which a real release must never do — say so, so a
                // test installer is never mistaken for a release one.
                log_warn!(
                    "BUILD_TRUSTS_DEVELOPMENT_KEY",
                    "this build trusts the development signing key and must not be distributed"
                );
            }
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
            // FR-088: elevate the sidecar only, never the interface (FR-030) — a dev/e2e build's
            // stand-in collector is exempt regardless (`collector_launcher` never elevates it).
            let advanced_access_enabled = app
                .state::<storage::AppState>()
                .storage
                .lock()
                .ok()
                .and_then(|storage| storage.advanced_access_enabled().ok())
                .unwrap_or(true);
            log_info!(
                "ADVANCED_ACCESS_STATE",
                "advanced access at startup (whether elevation succeeds is logged by the launch)",
                enabled = advanced_access_enabled,
                elevated = telemetry::launch::should_launch_elevated(advanced_access_enabled)
            );
            let elevated_connection: telemetry::launch::SharedElevatedConnection =
                std::sync::Arc::new(Mutex::new(None));
            let runtime = telemetry::runtime::CollectorRuntime::start(
                telemetry::launch::collector_launcher(
                    advanced_access_enabled,
                    elevated_connection.clone(),
                ),
                live.0,
                observer,
                config,
            )?;
            if initial_plan.is_some_and(|plan| plan.interval.is_none()) {
                runtime.set_battery_paused(true);
            }
            app.manage(CollectorHandle(Mutex::new(runtime)));
            app.manage(ElevatedConnectionHandle(elevated_connection));
            sampling_control::spawn_power_watch(app.handle().clone());
            let glass_thresholds = appearance::GlassThresholds::from_ruleset(&rules)
                .ok_or_else(|| std::io::Error::other("ruleset lacks the glass parameters"))?;
            let glass_monitor =
                std::sync::Arc::new(appearance::GlassMonitorState::new(glass_thresholds));
            app.manage(glass_monitor.clone());
            appearance::spawn_glass_monitor(app.handle().clone(), glass_monitor);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_live_snapshot,
            commands::get_coverage,
            commands::recheck_coverage,
            commands::get_onboarding_state,
            commands::set_onboarding_state,
            commands::get_window_state,
            commands::get_effective_locale,
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
            updates_app::install_update,
            appearance::report_glass_fps,
            appearance::get_effective_glass_level
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
    log_info!("APP_SHUTTING_DOWN", "stopping the collector and closing the session in progress");
    let Some(collector) = app.try_state::<CollectorHandle>() else { return };
    let Ok(mut runtime) = collector.0.lock() else { return };
    runtime.stop();
    drop(runtime);
    // Closes the elevated launcher's master pipe handles for real (ADR-0004 amendment,
    // 2026-09-28): this is the one case that is supposed to end the launcher process, not just a
    // collector session — dropped explicitly here rather than left to whenever the managed state
    // itself drops, so the launcher's own shutdown is not at the mercy of Tauri's teardown order.
    if let Some(elevated) = app.try_state::<ElevatedConnectionHandle>()
        && let Ok(mut connection) = elevated.0.lock()
    {
        *connection = None;
    }
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
        if let Err(error) = storage
            .prune_sessions(retention, &jiff::Timestamp::now().to_string())
            .and_then(|_| storage.enforce_session_cap(storage::Storage::MAX_RETAINED_SESSIONS))
        {
            log_warn!(
                component: "storage",
                "STORAGE_RETENTION_FAILED",
                format!("retention could not be applied on exit: {error}")
            );
        }
        // Last: with retention «solo esta sesión» this removes the very file the lines above went
        // to, so nothing is logged after it.
        if retention == "session"
            && let Ok(logs) = dev_faults::resolve_data_dir(app).map(|path| path.join("logs"))
        {
            let _ = logging::clear_directory(logs);
        }
    }
}
