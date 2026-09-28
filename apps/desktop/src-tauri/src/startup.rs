//! Integración de inicio con Windows para la preferencia `startup.enabled`, mediante el plugin
//! oficial `tauri-plugin-autostart` (clave HKCU de Run: sin elevación y reversible). La interfaz
//! no lo invoca: solo el backend, tras validar la preferencia.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;
use tauri_plugin_autostart::ManagerExt;

/// Enables or disables launching with Windows; disabling something that was never enabled is
/// not an error.
pub fn set_enabled(
    app: &tauri::AppHandle,
    enabled: bool,
) -> Result<(), tauri_plugin_autostart::Error> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else if manager.is_enabled()? {
        manager.disable()
    } else {
        Ok(())
    };
    match &result {
        Ok(()) => crate::log_info!("STARTUP_SET", "launch with Windows updated", enabled = enabled),
        Err(error) => crate::log_warn!(
            "STARTUP_SET_FAILED",
            format!("launch with Windows could not be updated: {error}"),
            enabled = enabled
        ),
    }
    result
}

pub fn apply_launch_mode(
    app: &tauri::AppHandle,
    values: &std::collections::BTreeMap<String, Value>,
) {
    let enabled = values.get("startup.enabled").and_then(Value::as_bool).unwrap_or(false);
    let tray = values.get("startup.mode").and_then(Value::as_str) == Some("tray");
    let monitoring =
        values.get("tray.monitoring_enabled").and_then(Value::as_bool).unwrap_or(false);
    if enabled
        && tray
        && monitoring
        && let Some(window) = tauri::Manager::get_webview_window(app, "main")
    {
        crate::log_info!("STARTUP_HIDDEN_IN_TRAY", "started hidden in the tray, as configured");
        if let Err(error) = window.hide() {
            crate::log_warn!(
                "STARTUP_HIDE_FAILED",
                format!("the window could not start hidden: {error}")
            );
        }
    }
}
