//! T019d / T-INT-003 (surface): the capability set in `capabilities/` is the whole
//! permission surface of the WebView. This test drives the real ACL through Tauri's
//! mock runtime so that additions or removals in `capabilities/default.json` are
//! caught here, not in production.

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn app_with_real_acl() -> (App<tauri::test::MockRuntime>, WebviewWindow<tauri::test::MockRuntime>) {
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![ping])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| panic!("mock app must build: {error}"));
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap_or_else(|error| panic!("main window must build: {error}"));
    (app, window)
}

fn invoke(window: &WebviewWindow<tauri::test::MockRuntime>, command: &str) -> Result<(), String> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap_or_else(|error| panic!("url: {error}")),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[test]
fn title_bar_window_commands_are_allowed() {
    let (_app, window) = app_with_real_acl();
    for command in [
        "plugin:window|minimize",
        "plugin:window|toggle_maximize",
        "plugin:window|internal_toggle_maximize",
        "plugin:window|is_maximized",
        "plugin:window|start_dragging",
    ] {
        let result = invoke(&window, command);
        assert!(result.is_ok(), "{command} must be allowed: {result:?}");
    }
}

#[test]
fn window_commands_outside_the_title_bar_are_denied() {
    let (_app, window) = app_with_real_acl();
    for command in [
        "plugin:window|set_size",
        "plugin:window|set_decorations",
        "plugin:window|set_always_on_top",
        "plugin:window|create",
        "plugin:webview|create_webview_window",
        "plugin:path|resolve_directory",
        "plugin:event|emit",
    ] {
        let error = invoke(&window, command).expect_err(command);
        assert!(error.contains("not allowed"), "{command} must be denied by the ACL: {error}");
    }
}

#[test]
fn application_commands_need_no_capability_entry() {
    let (_app, window) = app_with_real_acl();
    assert!(
        invoke(&window, "ping").is_ok(),
        "app commands are governed by generate_handler!, not by capabilities"
    );
}
