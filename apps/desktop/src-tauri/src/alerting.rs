//! Turns the live state into at most one prudent notification per episode (FR-025, T069/T070):
//! builds the observation for the [`AlertEngine`], applies the notification preferences and the
//! quiet period, and hands the texts (in the interface language) to a [`NotificationPort`].
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::diagnostics::Ruleset;
use crate::diagnostics::alerts::{AlertEngine, AlertEvent, AlertObservation, in_quiet_period};
use crate::diagnostics::rules::CoverageTier;
use crate::i18n::{self, Locale};
use crate::ports::{NotificationPort, PortError};
use crate::storage::AppState;
use crate::telemetry::live::LiveState;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// How long the notification preferences are trusted before they are read again.
const SETTINGS_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct Settings {
    enabled: bool,
    quiet_period: Value,
    locale: Locale,
    read_at: Instant,
}

/// The engine and the cached preferences it needs on every sample.
pub struct AlertHandle {
    engine: Mutex<Option<AlertEngine>>,
    settings: Mutex<Option<Settings>>,
}

impl AlertHandle {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(
                Ruleset::v1().ok().and_then(|rules| AlertEngine::from_ruleset(&rules)),
            ),
            settings: Mutex::new(None),
        }
    }

    /// Drops the cached preferences: the next sample reads them again (a setting just changed).
    pub fn forget_settings(&self) {
        if let Ok(mut settings) = self.settings.lock() {
            *settings = None;
        }
    }
}

impl Default for AlertHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn settings(app: &AppHandle, handle: &AlertHandle) -> Option<Settings> {
    let mut cached = handle.settings.lock().ok()?;
    if let Some(value) = cached.as_ref()
        && value.read_at.elapsed() < SETTINGS_TTL
    {
        return Some(value.clone());
    }
    let state = app.try_state::<AppState>()?;
    let preferences = state.storage.lock().ok()?.user_preferences().ok()?;
    let value = Settings {
        enabled: preferences.get("notifications.enabled").and_then(Value::as_bool).unwrap_or(false),
        quiet_period: preferences.get("notifications.quiet_period").cloned().unwrap_or(Value::Null),
        locale: i18n::current(preferences.get("locale.mode").and_then(Value::as_str)),
        read_at: Instant::now(),
    };
    *cached = Some(value.clone());
    Some(value)
}

/// The title and body of an alert, in `locale`.
pub fn message(locale: Locale, event: &AlertEvent) -> (String, String) {
    let key = event.kind.key();
    (
        i18n::text(locale, &format!("native.notification.{key}.title")),
        i18n::text(locale, &format!("native.notification.{key}.body")),
    )
}

/// Sends every raised alert through the port; the first failure stops nothing else (an alert that
/// cannot be shown is dropped, never retried into a burst).
pub fn dispatch(
    locale: Locale,
    events: &[AlertEvent],
    notifier: &mut dyn NotificationPort,
) -> Vec<PortError> {
    events
        .iter()
        .filter_map(|event| {
            let (title, body) = message(locale, event);
            notifier.notify(&title, &body).err()
        })
        .collect()
}

struct TauriNotifier<'a>(&'a AppHandle);

impl NotificationPort for TauriNotifier<'_> {
    fn notify(&mut self, title: &str, body: &str) -> Result<(), PortError> {
        use tauri_plugin_notification::NotificationExt;
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|_| PortError { code: "notification.failed" })
    }
}

/// A notification that is not an alert (the updater's «version available», asked for by turning
/// the updater on): same port, same wording rules, no engine.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let mut notifier = TauriNotifier(app);
    if let Err(error) = notifier.notify(title, body) {
        tracing::warn!(
            component = "core",
            code = error.code,
            msg = "a notification could not be shown"
        );
    }
}

fn raise(app: &AppHandle, locale: Locale, events: &[AlertEvent]) {
    if events.is_empty() {
        return;
    }
    let mut notifier = TauriNotifier(app);
    for error in dispatch(locale, events, &mut notifier) {
        tracing::warn!(
            component = "core",
            code = error.code,
            msg = "a notification could not be shown"
        );
    }
}

/// Feeds the engine with what the live state says right now. Called from the observer, which is
/// inside the live state's lock: `live` is passed in.
pub fn observe(app: &AppHandle, live: &LiveState, session_id: Option<&str>) {
    let Some(handle) = app.try_state::<AlertHandle>() else { return };
    let Some(settings) = settings(app, &handle) else { return };
    let diagnostic = live.diagnostic();
    let tier = live.coverage_signals().tier();
    let now_ms = crate::commands::epoch_ms();
    let silenced = in_quiet_period(
        &settings.quiet_period,
        u8::try_from(jiff::Zoned::now().hour()).unwrap_or(0),
    );
    let observation = AlertObservation {
        at_ms: now_ms,
        classification: diagnostic.map(|result| result.classification.key()),
        severity: diagnostic.and_then(|result| result.severity).map(|value| value.key()),
        certainty: diagnostic
            .map(|_| if tier == CoverageTier::A { "observed" } else { "inferred" }),
        collector_state: live.collector().as_str(),
        guided_finished: false,
        notifications_enabled: settings.enabled,
        silenced,
        session_id,
        event_id: None,
    };
    let events = {
        let Ok(mut engine) = handle.engine.lock() else { return };
        let Some(engine) = engine.as_mut() else { return };
        engine.observe(observation)
    };
    raise(app, settings.locale, &events);
}

/// A guided test ended: one notification, immediately (it is an event, not an episode).
pub fn guided_finished(app: &AppHandle, session_id: &str) {
    let Some(handle) = app.try_state::<AlertHandle>() else { return };
    let Some(settings) = settings(app, &handle) else { return };
    let observation = AlertObservation {
        at_ms: crate::commands::epoch_ms(),
        collector_state: "running",
        guided_finished: true,
        notifications_enabled: settings.enabled,
        silenced: in_quiet_period(
            &settings.quiet_period,
            u8::try_from(jiff::Zoned::now().hour()).unwrap_or(0),
        ),
        session_id: Some(session_id),
        ..AlertObservation::default()
    };
    let events = {
        let Ok(mut engine) = handle.engine.lock() else { return };
        let Some(engine) = engine.as_mut() else { return };
        engine.observe(observation)
    };
    raise(app, settings.locale, &events);
}

#[cfg(test)]
mod tests {
    use super::{dispatch, message};
    use crate::diagnostics::alerts::{AlertEvent, AlertKind};
    use crate::i18n::Locale;
    use crate::ports::{NotificationPort, PortError};

    #[derive(Default)]
    struct Fake {
        shown: Vec<(String, String)>,
        fail: bool,
    }

    impl NotificationPort for Fake {
        fn notify(&mut self, title: &str, body: &str) -> Result<(), PortError> {
            if self.fail {
                return Err(PortError { code: "notification.failed" });
            }
            self.shown.push((title.to_owned(), body.to_owned()));
            Ok(())
        }
    }

    fn event(kind: AlertKind) -> AlertEvent {
        AlertEvent { kind, at_ms: 0, session_id: None, event_id: None }
    }

    #[test]
    fn every_alert_kind_has_prudent_text_in_both_languages() {
        for kind in AlertKind::ALL {
            for locale in [Locale::Es, Locale::En] {
                let (title, body) = message(locale, &event(kind));
                assert!(!title.starts_with("native."), "{kind:?}: {title}");
                assert!(!body.starts_with("native."), "{kind:?}: {body}");
                assert!(!title.is_empty() && !body.is_empty());
            }
        }
    }

    #[test]
    fn the_wording_never_states_a_cause_as_a_certainty() {
        let (_, body) = message(Locale::En, &event(AlertKind::ThermalConfirmed));
        let (title, _) = message(Locale::En, &event(AlertKind::ThermalConfirmed));
        assert!(title.starts_with("Possible"), "{title}");
        assert!(body.contains("evidence"), "{body}");
    }

    #[test]
    fn alerts_reach_the_port_in_the_interface_language() {
        let mut fake = Fake::default();
        let errors = dispatch(
            Locale::Es,
            &[event(AlertKind::GuidedFinished), event(AlertKind::CollectorLost)],
            &mut fake,
        );
        assert!(errors.is_empty());
        assert_eq!(fake.shown.len(), 2);
        assert_eq!(fake.shown[0].0, "Diagnóstico guiado terminado");
        assert_eq!(fake.shown[1].0, "ThrottleWatch dejó de recibir datos");
    }

    #[test]
    fn a_notification_that_cannot_be_shown_is_reported_not_retried() {
        let mut fake = Fake { fail: true, ..Fake::default() };
        let errors = dispatch(Locale::En, &[event(AlertKind::ThermalConfirmed)], &mut fake);
        assert_eq!(errors.len(), 1);
        assert!(fake.shown.is_empty());
    }
}
