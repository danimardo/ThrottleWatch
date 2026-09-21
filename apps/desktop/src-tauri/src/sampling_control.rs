//! Applies the sampling preferences to the running collector (FR-060, T071): the profile picks
//! the interval, `sampling.on_battery` decides what happens away from AC, and a change of either
//! or of the power source reaches the collector without restarting the application.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use crate::diagnostics::Ruleset;
use crate::diagnostics::sampling::{OnBatteryMode, SamplingPlan, SamplingProfile, plan};
use crate::storage::AppState;
use crate::telemetry::power_context::{PowerSource, read_windows_power_context};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// How often the power source is looked at; a change is applied within this delay.
const POWER_POLL: Duration = Duration::from_secs(2);

/// The plan the stored preferences ask for. Unknown or missing values fall back to the initial
/// ones (`normal`, `keep`, no per-core history), never to an error.
pub fn plan_from_preferences(
    rules: &Ruleset,
    preferences: &BTreeMap<String, Value>,
    ac_power: bool,
) -> Option<SamplingPlan> {
    let profile = preferences
        .get("sampling.profile")
        .and_then(Value::as_str)
        .and_then(SamplingProfile::from_key)
        .unwrap_or(SamplingProfile::Normal);
    let on_battery = preferences
        .get("sampling.on_battery")
        .and_then(Value::as_str)
        .and_then(OnBatteryMode::from_key)
        .unwrap_or(OnBatteryMode::Keep);
    let per_core =
        preferences.get("sampling.per_core_history").and_then(Value::as_bool).unwrap_or(false);
    plan(rules, profile, on_battery, ac_power, per_core)
}

/// Whether the machine runs on AC. An unknown source is treated as AC: the conservative choice
/// never pauses or slows sampling on a guess.
pub fn on_ac_power() -> bool {
    read_windows_power_context().is_none_or(|context| context.source != PowerSource::Battery)
}

/// Reads the preferences and the power source and pushes the resulting plan to the collector.
pub fn apply(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let Ok(preferences) = state.storage.lock().map(|storage| storage.user_preferences()) else {
        return;
    };
    let Ok(preferences) = preferences else { return };
    let Ok(rules) = Ruleset::v1() else { return };
    let Some(sampling) = plan_from_preferences(&rules, &preferences, on_ac_power()) else {
        return;
    };
    let Some(handle) = app.try_state::<crate::CollectorHandle>() else { return };
    let Ok(runtime) = handle.0.lock() else { return };
    match sampling.interval {
        Some(interval) => {
            runtime.set_interval(interval);
            runtime.set_battery_paused(false);
        }
        None => runtime.set_battery_paused(true),
    }
}

/// Watches the power source for the life of the process and re-applies the plan when it changes.
pub fn spawn_power_watch(app: AppHandle) {
    let _ = std::thread::Builder::new().name("power-watch".to_owned()).spawn(move || {
        let mut previous = on_ac_power();
        loop {
            std::thread::sleep(POWER_POLL);
            let current = on_ac_power();
            if current != previous {
                previous = current;
                apply(&app);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::plan_from_preferences;
    use crate::diagnostics::Ruleset;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::time::Duration;

    fn rules() -> Ruleset {
        Ruleset::v1().unwrap_or_else(|error| panic!("ruleset: {error}"))
    }

    fn prefs(profile: &str, on_battery: &str) -> BTreeMap<String, serde_json::Value> {
        [
            ("sampling.profile".to_owned(), json!(profile)),
            ("sampling.on_battery".to_owned(), json!(on_battery)),
            ("sampling.per_core_history".to_owned(), json!(true)),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn the_profile_sets_the_interval_on_ac() {
        let rules = rules();
        let interval = |profile| {
            plan_from_preferences(&rules, &prefs(profile, "keep"), true).and_then(|p| p.interval)
        };
        assert_eq!(interval("low_power"), Some(Duration::from_secs(5)));
        assert_eq!(interval("normal"), Some(Duration::from_secs(1)));
        assert_eq!(interval("diagnostic"), Some(Duration::from_millis(500)));
    }

    #[test]
    fn away_from_ac_the_battery_policy_decides() {
        let rules = rules();
        let on_battery = |mode| {
            plan_from_preferences(&rules, &prefs("diagnostic", mode), false)
                .and_then(|p| p.interval)
        };
        assert_eq!(on_battery("keep"), Some(Duration::from_millis(500)));
        assert_eq!(on_battery("low_power"), Some(Duration::from_secs(5)));
        assert_eq!(on_battery("pause"), None);
    }

    #[test]
    fn missing_or_unknown_preferences_fall_back_to_the_initial_values() {
        let rules = rules();
        let empty = BTreeMap::new();
        let plan = plan_from_preferences(&rules, &empty, false);
        assert_eq!(plan.and_then(|p| p.interval), Some(Duration::from_secs(1)));
        assert!(!plan.is_some_and(|p| p.per_core_history));
        let invalid = prefs("turbo", "explode");
        assert_eq!(
            plan_from_preferences(&rules, &invalid, false).and_then(|p| p.interval),
            Some(Duration::from_secs(1))
        );
    }
}
