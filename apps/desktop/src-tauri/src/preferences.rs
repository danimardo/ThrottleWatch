#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use serde_json::{Value, json};

pub const SCHEMA_VERSION: u32 = 1;

pub type PreferenceMap = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceError {
    UnknownKey,
    InvalidValue,
    Dependency,
}

pub fn default_values() -> PreferenceMap {
    BTreeMap::from([
        ("locale.mode".to_owned(), json!("system")),
        ("appearance.theme".to_owned(), json!("system")),
        ("appearance.motion".to_owned(), json!("system")),
        ("appearance.glass".to_owned(), json!("system")),
        ("sampling.profile".to_owned(), json!("normal")),
        ("sampling.on_battery".to_owned(), json!("keep")),
        ("sampling.per_core_history".to_owned(), json!(false)),
        ("history.retention".to_owned(), json!("7d")),
        ("notifications.enabled".to_owned(), json!(false)),
        ("notifications.quiet_period".to_owned(), Value::Null),
        ("tray.monitoring_enabled".to_owned(), json!(false)),
        ("lifecycle.close_action".to_owned(), json!("unset")),
        ("startup.enabled".to_owned(), json!(false)),
        ("startup.mode".to_owned(), json!("window")),
        ("guided.duration".to_owned(), json!("standard")),
        ("guided.require_ac".to_owned(), json!(false)),
        ("guided.notify_on_finish".to_owned(), json!(false)),
        ("privacy.anonymize_exports".to_owned(), json!(true)),
        ("updates.enabled".to_owned(), json!(false)),
        ("logging.detailed_until".to_owned(), Value::Null),
    ])
}

pub fn update(
    current: &PreferenceMap,
    key: &str,
    value: Value,
    now: &str,
) -> Result<(PreferenceMap, Vec<String>), PreferenceError> {
    let mut next = current.clone();
    let value = normalise_value(key, value, now)?;
    next.insert(key.to_owned(), value);

    if key == "tray.monitoring_enabled" && next.get(key) == Some(&Value::Bool(false)) {
        if next.get("startup.mode").and_then(Value::as_str) == Some("tray") {
            next.insert("startup.mode".to_owned(), json!("window"));
        }
        if next.get("lifecycle.close_action").and_then(Value::as_str) == Some("tray") {
            next.insert("lifecycle.close_action".to_owned(), json!("exit"));
        }
    }
    // FR-046: choosing "continue in the tray" on the first close (or from Settings) must turn
    // monitoring in the tray on — there is no "tray without sampling" configuration.
    if key == "lifecycle.close_action" && next.get(key).and_then(Value::as_str) == Some("tray") {
        next.insert("tray.monitoring_enabled".to_owned(), json!(true));
    }

    if next.get("startup.mode").and_then(Value::as_str) == Some("tray")
        && next.get("tray.monitoring_enabled") != Some(&Value::Bool(true))
    {
        return Err(PreferenceError::Dependency);
    }
    if next.get("guided.notify_on_finish") == Some(&Value::Bool(true))
        && next.get("notifications.enabled") != Some(&Value::Bool(true))
    {
        return Err(PreferenceError::Dependency);
    }

    let adjusted = current
        .iter()
        .filter(|(name, before)| next.get(*name) != Some(*before))
        .map(|(name, _before)| name.clone())
        .chain(next.keys().filter(|name| !current.contains_key(*name)).cloned())
        .filter(|name| name != key)
        .collect();
    Ok((next, adjusted))
}

/// Returns whether detailed logging is still inside its explicit 24-hour window.
/// Expired values are treated as disabled without mutating preferences on read.
pub fn detailed_logging_enabled(values: &PreferenceMap, now: &str) -> bool {
    let Some(until) = values.get("logging.detailed_until").and_then(Value::as_str) else {
        return false;
    };
    let Ok(until) = until.parse::<jiff::Timestamp>() else {
        return false;
    };
    let Ok(now) = now.parse::<jiff::Timestamp>() else {
        return false;
    };
    until > now
}

fn normalise_value(key: &str, value: Value, _now: &str) -> Result<Value, PreferenceError> {
    let valid = match key {
        "locale.mode" => one_of(&value, &["system", "es", "en"]),
        "appearance.theme" => one_of(&value, &["system", "light", "dark"]),
        "appearance.motion" => one_of(&value, &["system", "reduced", "full"]),
        "appearance.glass" => one_of(&value, &["system", "full", "reduced", "off"]),
        "sampling.profile" => one_of(&value, &["low_power", "normal", "diagnostic"]),
        "sampling.on_battery" => one_of(&value, &["keep", "low_power", "pause"]),
        "history.retention" => one_of(&value, &["session", "1d", "7d", "30d"]),
        "lifecycle.close_action" => one_of(&value, &["unset", "exit", "tray"]),
        "startup.mode" => one_of(&value, &["window", "tray"]),
        "guided.duration" => one_of(&value, &["short", "standard", "long"]),
        "sampling.per_core_history"
        | "notifications.enabled"
        | "tray.monitoring_enabled"
        | "startup.enabled"
        | "guided.require_ac"
        | "guided.notify_on_finish"
        | "privacy.anonymize_exports"
        | "updates.enabled" => value.is_boolean(),
        "notifications.quiet_period" => quiet_period(&value),
        "logging.detailed_until" => {
            if value == Value::Bool(true) {
                return Ok(json!(
                    jiff::Timestamp::now()
                        .checked_add(jiff::SignedDuration::from_hours(24))
                        .map_err(|_| PreferenceError::InvalidValue)?
                        .to_string()
                ));
            }
            if value == Value::Bool(false) || value.is_null() {
                return Ok(Value::Null);
            }
            value.as_str().is_some()
        }
        _ => return Err(PreferenceError::UnknownKey),
    };
    valid.then_some(value).ok_or(PreferenceError::InvalidValue)
}

fn one_of(value: &Value, allowed: &[&str]) -> bool {
    value.as_str().is_some_and(|candidate| allowed.contains(&candidate))
}

fn quiet_period(value: &Value) -> bool {
    let Some(period) = value.as_object() else {
        return value.is_null();
    };
    let hour = |key: &str| {
        period
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|value| value.trim().parse::<u8>().is_ok_and(|hour| hour < 24))
    };
    period.len() == 2 && hour("start") && hour("end")
}

#[cfg(test)]
mod tests {
    use super::{PreferenceError, default_values, detailed_logging_enabled, update};
    use serde_json::json;

    #[test]
    fn defaults_cover_every_known_preference() {
        assert_eq!(default_values().len(), 20);
        assert_eq!(default_values()["privacy.anonymize_exports"], json!(true));
    }

    #[test]
    fn disabling_tray_repairs_incompatible_startup_atomically() {
        let mut values = default_values();
        values.insert("tray.monitoring_enabled".to_owned(), json!(true));
        values.insert("startup.mode".to_owned(), json!("tray"));
        values.insert("lifecycle.close_action".to_owned(), json!("tray"));
        let result = update(&values, "tray.monitoring_enabled", json!(false), "now");
        assert!(result.is_ok());
        let (next, adjusted) = result.unwrap_or_default();
        assert_eq!(next["startup.mode"], json!("window"));
        assert_eq!(next["lifecycle.close_action"], json!("exit"));
        assert_eq!(adjusted, vec!["lifecycle.close_action", "startup.mode"]);
    }

    #[test]
    fn choosing_tray_as_the_close_action_turns_tray_monitoring_on() {
        let values = default_values();
        assert_eq!(values["tray.monitoring_enabled"], json!(false));
        let result = update(&values, "lifecycle.close_action", json!("tray"), "now");
        let (next, adjusted) = result.unwrap_or_default();
        assert_eq!(next["tray.monitoring_enabled"], json!(true));
        assert!(adjusted.contains(&"tray.monitoring_enabled".to_owned()));
    }

    #[test]
    fn rejects_invalid_dependencies_and_unknown_keys() {
        let values = default_values();
        assert_eq!(
            update(&values, "startup.mode", json!("tray"), "now"),
            Err(PreferenceError::Dependency)
        );
        assert_eq!(
            update(&values, "unknown.key", json!(true), "now"),
            Err(PreferenceError::UnknownKey)
        );
    }

    #[test]
    fn detailed_logging_is_a_24_hour_expiry() {
        let result = update(&default_values(), "logging.detailed_until", json!(true), "now");
        assert!(result.is_ok());
        let (next, _) = result.unwrap_or_default();
        assert!(next["logging.detailed_until"].as_str().is_some());
    }

    #[test]
    fn detailed_logging_expires_without_a_write() {
        let mut values = default_values();
        values.insert("logging.detailed_until".to_owned(), json!("2026-09-20T12:00:00Z"));
        assert!(detailed_logging_enabled(&values, "2026-09-20T11:59:59Z"));
        assert!(!detailed_logging_enabled(&values, "2026-09-20T12:00:00Z"));
        assert!(!detailed_logging_enabled(&values, "not-a-timestamp"));
    }

    #[test]
    fn disabling_detailed_logging_persists_an_empty_deadline() {
        let mut values = default_values();
        values.insert("logging.detailed_until".to_owned(), json!("2026-09-20T12:00:00Z"));
        let result = update(&values, "logging.detailed_until", json!(false), "now");
        assert!(result.is_ok());
        let (next, _) = result.unwrap_or_default();
        assert!(next["logging.detailed_until"].is_null());
    }
}
