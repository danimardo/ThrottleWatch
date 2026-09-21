//! The texts Rust shows on its own (tray menu, tooltips, native notifications) come from the same
//! `es`/`en` catalogs as the interface (`native.*` keys): one source, one parity check, no literals
//! in the backend (FR-033).
#![deny(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;
use std::sync::OnceLock;

const ES: &str = include_str!("../../src/lib/i18n/locales/es.json");
const EN: &str = include_str!("../../src/lib/i18n/locales/en.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Es,
    En,
}

/// `locale.mode` (`system`, `es`, `en`) and the Windows locale name to the interface language:
/// `es`, `ca`, `gl`, `eu`, `ast` and `an` resolve to Spanish, everything else (including `pt`)
/// to English. The same rule as `lib/i18n/index.ts`.
pub fn resolve(mode: &str, windows_locale: &str) -> Locale {
    match mode {
        "es" => Locale::Es,
        "en" => Locale::En,
        _ => {
            let normalized = windows_locale.trim().to_lowercase().replace('_', "-");
            let language = normalized.split('-').next().unwrap_or("");
            if ["es", "ca", "gl", "eu", "ast", "an"].contains(&language) {
                Locale::Es
            } else {
                Locale::En
            }
        }
    }
}

/// The locale name Windows reports for the user (`es-ES`); empty if it cannot be read.
#[cfg(windows)]
pub fn windows_locale() -> String {
    let mut buffer = [0_u16; 85];
    // SAFETY: the buffer is writable for its whole length and Windows NUL-terminates what it
    // writes; the returned count includes the terminator.
    let written = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buffer) };
    let length = usize::try_from(written).unwrap_or(0).saturating_sub(1);
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}

#[cfg(not(windows))]
pub fn windows_locale() -> String {
    String::new()
}

fn catalog(locale: Locale) -> &'static Value {
    static CATALOGS: OnceLock<[Value; 2]> = OnceLock::new();
    let catalogs = CATALOGS.get_or_init(|| {
        [
            serde_json::from_str(ES).unwrap_or(Value::Null),
            serde_json::from_str(EN).unwrap_or(Value::Null),
        ]
    });
    match locale {
        Locale::Es => &catalogs[0],
        Locale::En => &catalogs[1],
    }
}

fn lookup(locale: Locale, key: &str) -> Option<&'static str> {
    key.split('.').try_fold(catalog(locale), |node, segment| node.get(segment))?.as_str()
}

/// The text for `key` in `locale`. A key missing from the catalog shows the key itself, which a
/// test (`every_native_key_exists_in_both_catalogs`) makes impossible to ship.
pub fn text(locale: Locale, key: &str) -> String {
    lookup(locale, key).unwrap_or(key).to_owned()
}

/// The locale in force: the stored `locale.mode`, resolved against Windows' language.
pub fn current(mode: Option<&str>) -> Locale {
    resolve(mode.unwrap_or("system"), &windows_locale())
}

#[cfg(test)]
mod tests {
    use super::{Locale, lookup, resolve, text};

    #[test]
    fn the_language_follows_the_mode_or_windows() {
        assert_eq!(resolve("es", "en-US"), Locale::Es);
        assert_eq!(resolve("en", "es-ES"), Locale::En);
        for regional in ["es-ES", "ca-ES", "gl-ES", "eu-ES", "ast-ES", "an-ES", "ES_es"] {
            assert_eq!(resolve("system", regional), Locale::Es, "{regional}");
        }
        for other in ["en-US", "pt-PT", "fr-FR", "", "de"] {
            assert_eq!(resolve("system", other), Locale::En, "{other}");
        }
    }

    const NATIVE_KEYS: [&str; 29] = [
        "native.tray.status",
        "native.tray.open",
        "native.tray.pause",
        "native.tray.resume",
        "native.tray.quit",
        "native.tray.tooltip",
        "native.tray.state.normal",
        "native.tray.state.warning",
        "native.tray.state.critical",
        "native.tray.state.unknown",
        "native.tray.state.disconnected",
        "native.notification.thermal_confirmed.title",
        "native.notification.thermal_confirmed.body",
        "native.notification.power_limited.title",
        "native.notification.power_limited.body",
        "native.notification.platform_limited.title",
        "native.notification.platform_limited.body",
        "native.notification.collector_lost.title",
        "native.notification.guided_finished.title",
        "native.live.cpu_unknown",
        "native.live.power.ac",
        "native.live.power.battery",
        "native.live.power.unknown",
        "native.live.confidence.low",
        "native.live.confidence.medium",
        "native.live.confidence.high",
        "native.live.topology_waiting",
        "native.live.topology_virtualized_suffix",
        "native.live.topology_logical_processors",
    ];

    #[test]
    fn every_native_key_exists_in_both_catalogs() {
        for key in NATIVE_KEYS {
            for locale in [Locale::Es, Locale::En] {
                assert!(lookup(locale, key).is_some_and(|value| !value.is_empty()), "{key}");
            }
        }
        for kind in ["collector_lost", "guided_finished"] {
            for locale in [Locale::Es, Locale::En] {
                assert!(lookup(locale, &format!("native.notification.{kind}.body")).is_some());
            }
        }
    }

    #[test]
    fn texts_come_in_the_requested_language_and_a_missing_key_shows_itself() {
        assert_eq!(text(Locale::Es, "native.tray.quit"), "Salir");
        assert_eq!(text(Locale::En, "native.tray.quit"), "Quit");
        assert_eq!(text(Locale::En, "native.tray.nope"), "native.tray.nope");
    }
}
