//! The tray icon and its menu (FR-059): five states mapped from the live classification and the
//! collector, menu and tooltip in the interface language, click to show or hide the window.
//! `state_for` and `overlay_dot` are pure; `setup` and `refresh` are the thin Tauri glue.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::commands::{LiveHandle, TrayController};
use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
use crate::diagnostics::rules::CoverageTier;
use crate::i18n::{self, Locale};
use crate::storage::AppState;
use crate::telemetry::live::{CollectorState, LiveState};
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconState {
    Normal,
    Warning,
    Critical,
    Unknown,
    Disconnected,
}

impl TrayIconState {
    /// The identifier shared with the bridge (`tray:state`, `trayStateSchema`).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Unknown => "unknown",
            Self::Disconnected => "disconnected",
        }
    }

    /// The dot painted over the application icon. The tooltip and the menu say the same in
    /// words, so the state never depends on colour alone.
    const fn color(self) -> [u8; 4] {
        match self {
            Self::Normal => [0x2e, 0x9e, 0x5b, 0xff],
            Self::Warning => [0xe0, 0xa1, 0x00, 0xff],
            Self::Critical => [0xd1, 0x34, 0x38, 0xff],
            Self::Unknown => [0x8a, 0x8a, 0x8a, 0xff],
            Self::Disconnected => [0x3a, 0x3a, 0x3a, 0xff],
        }
    }
}

/// Maps the live classification and the collector to the tray state. Only a limitation observed
/// directly (coverage tier A, below the guaranteed frequency) is `critical`; an inferred one, a
/// hot machine without proof or a limitation above base speed is a `warning`.
pub fn state_for(
    collector: CollectorState,
    paused: bool,
    diagnostic: Option<&DiagnosticResult>,
    tier: CoverageTier,
) -> TrayIconState {
    if matches!(collector, CollectorState::Failed | CollectorState::Stopped) {
        return TrayIconState::Disconnected;
    }
    if paused || matches!(collector, CollectorState::Starting | CollectorState::Restarting) {
        return TrayIconState::Unknown;
    }
    let Some(result) = diagnostic else { return TrayIconState::Unknown };
    match result.classification {
        Classification::Indeterminate => TrayIconState::Unknown,
        Classification::Normal => TrayIconState::Normal,
        Classification::HotUnproven | Classification::ThermalProbable => TrayIconState::Warning,
        Classification::ThermalConfirmed
        | Classification::PowerLimited
        | Classification::PlatformLimited
        | Classification::MixedLimit => {
            if result.severity == Some(Severity::BelowBase) && tier == CoverageTier::A {
                TrayIconState::Critical
            } else {
                TrayIconState::Warning
            }
        }
    }
}

/// Paints a filled dot in the bottom-right quarter of an RGBA image.
pub fn overlay_dot(rgba: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    let radius = f64::from(width.min(height)) * 0.27;
    let center_x = f64::from(width) - radius - 1.0;
    let center_y = f64::from(height) - radius - 1.0;
    for y in 0..height {
        for x in 0..width {
            let dx = f64::from(x) + 0.5 - center_x;
            let dy = f64::from(y) + 0.5 - center_y;
            if dx * dx + dy * dy <= radius * radius {
                let at = ((y * width + x) * 4) as usize;
                if let Some(pixel) = rgba.get_mut(at..at + 4) {
                    pixel.copy_from_slice(&color);
                }
            }
        }
    }
}

struct Applied {
    state: TrayIconState,
    paused: bool,
    locale: Locale,
}

/// The tray's live pieces: kept so a state, a pause or a language change only rewrites them.
pub struct TrayUi {
    icon: TrayIcon<Wry>,
    status: MenuItem<Wry>,
    open: MenuItem<Wry>,
    pause: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    applied: Mutex<Option<Applied>>,
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let locale = current_locale(app.handle());
    let label = |key: &str| i18n::text(locale, key);
    let status =
        MenuItem::with_id(app, "status", label("native.tray.status"), false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", label("native.tray.open"), true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", label("native.tray.pause"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label("native.tray.quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&status, &open, &pause, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| std::io::Error::other("default tray icon is missing"))?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "status" | "open" => show_main_window(app),
            "pause" => {
                let controller = app.state::<TrayController>();
                let runtime = app.state::<crate::CollectorHandle>();
                let paused = !controller.paused.load(Ordering::SeqCst);
                let _ = crate::commands::apply_tray_pause(app, &controller, &runtime, paused);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, .. }) {
                let app = tray.app_handle();
                let Some(window) = app.get_webview_window("main") else {
                    return;
                };
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    show_main_window(app);
                }
            }
        })
        .build(app)?;
    app.manage(TrayUi { icon: tray, status, open, pause, quit, applied: Mutex::new(None) });
    refresh(app.handle(), None, true);
    Ok(())
}

pub fn current_locale(app: &AppHandle) -> Locale {
    let mode = app.try_state::<AppState>().and_then(|state| {
        state.storage.lock().ok().and_then(|storage| storage.user_preferences().ok()).and_then(
            |preferences| {
                preferences.get("locale.mode").and_then(|value| value.as_str().map(str::to_owned))
            },
        )
    });
    i18n::current(mode.as_deref())
}

fn state_from(live: &LiveState, paused: bool) -> TrayIconState {
    state_for(live.collector(), paused, live.diagnostic(), live.coverage_signals().tier())
}

/// The state the tray would show right now. Called from the observer, which is already inside
/// the live state's lock, `live` is passed in instead of being locked again.
pub fn current_state(app: &AppHandle, live: Option<&LiveState>) -> TrayIconState {
    let paused = app
        .try_state::<TrayController>()
        .is_some_and(|controller| controller.paused.load(Ordering::SeqCst));
    match (live, app.try_state::<LiveHandle>()) {
        (Some(live), _) => state_from(live, paused),
        (None, Some(handle)) => handle.read(|live| state_from(live, paused)),
        (None, None) => TrayIconState::Unknown,
    }
}

/// Rewrites the icon, tooltip and menu texts when the state, the pause or the language changed
/// (`force` after a language change: the storage read it needs is skipped otherwise).
pub fn refresh(app: &AppHandle, live: Option<&LiveState>, force: bool) {
    let Some(ui) = app.try_state::<TrayUi>() else { return };
    let state = current_state(app, live);
    let paused = app
        .try_state::<TrayController>()
        .is_some_and(|controller| controller.paused.load(Ordering::SeqCst));
    let previous = ui.applied.lock().ok().and_then(|applied| {
        applied.as_ref().map(|value| (value.state, value.paused, value.locale))
    });
    if !force && previous.is_some_and(|(s, p, _)| s == state && p == paused) {
        return;
    }
    let locale = if force || previous.is_none() {
        current_locale(app)
    } else {
        previous.map_or(Locale::En, |(_, _, locale)| locale)
    };
    let label = |key: &str| i18n::text(locale, key);
    let state_text = label(&format!("native.tray.state.{}", state.key()));
    let _ = ui.status.set_text(format!("{}: {state_text}", label("native.tray.status")));
    let _ = ui.open.set_text(label("native.tray.open"));
    let _ =
        ui.pause.set_text(label(if paused { "native.tray.resume" } else { "native.tray.pause" }));
    let _ = ui.quit.set_text(label("native.tray.quit"));
    let _ = ui.icon.set_tooltip(Some(format!("{} — {state_text}", label("native.tray.tooltip"))));
    if let Some(base) = app.default_window_icon() {
        let (width, height) = (base.width(), base.height());
        let mut rgba = base.rgba().to_vec();
        overlay_dot(&mut rgba, width, height, state.color());
        let _ = ui.icon.set_icon(Some(Image::new_owned(rgba, width, height)));
    }
    if let Ok(mut applied) = ui.applied.lock() {
        *applied = Some(Applied { state, paused, locale });
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayIconState, overlay_dot, state_for};
    use crate::diagnostics::classifier::{Classification, DiagnosticResult, Severity};
    use crate::diagnostics::rules::CoverageTier;
    use crate::telemetry::live::CollectorState;

    fn result(classification: Classification, severity: Option<Severity>) -> DiagnosticResult {
        DiagnosticResult {
            classification,
            severity,
            coverage: CoverageTier::A,
            confidence: 0.8,
            platform: None,
            evidence: vec![],
            alternative_causes: vec![],
            analyzed_from_ms: None,
            analyzed_to_ms: None,
            windows: 1,
        }
    }

    fn state(
        collector: CollectorState,
        paused: bool,
        diagnostic: Option<&DiagnosticResult>,
        tier: CoverageTier,
    ) -> TrayIconState {
        state_for(collector, paused, diagnostic, tier)
    }

    #[test]
    fn a_lost_collector_is_disconnected_whatever_else_is_known() {
        let critical = result(Classification::ThermalConfirmed, Some(Severity::BelowBase));
        for collector in [CollectorState::Failed, CollectorState::Stopped] {
            assert_eq!(
                state(collector, false, Some(&critical), CoverageTier::A),
                TrayIconState::Disconnected
            );
        }
    }

    #[test]
    fn pausing_or_starting_or_having_no_verdict_is_unknown() {
        let normal = result(Classification::Normal, None);
        assert_eq!(
            state(CollectorState::Running, true, Some(&normal), CoverageTier::A),
            TrayIconState::Unknown
        );
        assert_eq!(
            state(CollectorState::Starting, false, Some(&normal), CoverageTier::A),
            TrayIconState::Unknown
        );
        assert_eq!(
            state(CollectorState::Running, false, None, CoverageTier::A),
            TrayIconState::Unknown
        );
        let unknown = result(Classification::Indeterminate, None);
        assert_eq!(
            state(CollectorState::Running, false, Some(&unknown), CoverageTier::A),
            TrayIconState::Unknown
        );
    }

    #[test]
    fn only_a_directly_observed_limitation_below_base_is_critical() {
        let below = result(Classification::ThermalConfirmed, Some(Severity::BelowBase));
        assert_eq!(
            state(CollectorState::Running, false, Some(&below), CoverageTier::A),
            TrayIconState::Critical
        );
        assert_eq!(
            state(CollectorState::Running, false, Some(&below), CoverageTier::B),
            TrayIconState::Warning,
            "an inferred class never alarms"
        );
        let boost = result(Classification::PowerLimited, Some(Severity::Boost));
        assert_eq!(
            state(CollectorState::Running, false, Some(&boost), CoverageTier::A),
            TrayIconState::Warning
        );
        for warning in [Classification::HotUnproven, Classification::ThermalProbable] {
            assert_eq!(
                state(
                    CollectorState::Running,
                    false,
                    Some(&result(warning, None)),
                    CoverageTier::B
                ),
                TrayIconState::Warning
            );
        }
        assert_eq!(
            state(
                CollectorState::Running,
                false,
                Some(&result(Classification::Normal, None)),
                CoverageTier::C
            ),
            TrayIconState::Normal
        );
    }

    #[test]
    fn the_dot_lands_in_the_bottom_right_and_leaves_the_rest_of_the_icon_alone() {
        let (width, height) = (32_u32, 32_u32);
        let mut rgba = vec![0x11_u8; (width * height * 4) as usize];
        overlay_dot(&mut rgba, width, height, [1, 2, 3, 255]);
        let pixel = |x: u32, y: u32| {
            let at = ((y * width + x) * 4) as usize;
            [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
        };
        assert_eq!(pixel(24, 24), [1, 2, 3, 255]);
        assert_eq!(pixel(2, 2), [0x11; 4]);
        assert_eq!(pixel(2, 28), [0x11; 4]);
    }

    #[test]
    fn every_state_has_its_own_key_and_colour() {
        let all = [
            TrayIconState::Normal,
            TrayIconState::Warning,
            TrayIconState::Critical,
            TrayIconState::Unknown,
            TrayIconState::Disconnected,
        ];
        let keys: std::collections::BTreeSet<_> = all.iter().map(|s| s.key()).collect();
        let colors: std::collections::BTreeSet<_> = all.iter().map(|s| s.color()).collect();
        assert_eq!((keys.len(), colors.len()), (5, 5));
    }
}
