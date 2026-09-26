//! `TW_DEV_*` fault injection for E2E-13 (HU-19): forces the two storage failures FR-075
//! describes so the real application can be watched surviving them, instead of trusting unit
//! tests and a fake bridge.
//!
//! Compiled only into debug and `e2e` builds, exactly like [`crate::config::DevConfig`]: a
//! release build has no code path that reads these variables, so it cannot be faulted even if
//! someone sets them.
//!
//! | Variable | Efecto |
//! |---|---|
//! | `TW_DEV_STORAGE_FAIL_WRITES=<n>` | las `n` primeras escrituras de sesión fallan; después vuelven a funcionar, así que una sola ejecución enseña el aviso apareciendo **y** desapareciendo |
//! | `TW_DEV_CORRUPT_DB=1` | escribe basura en el fichero de base de datos antes de abrirlo, para que el arranque tenga que apartarlo y crear uno nuevo |
//! | `TW_DEV_COLLECTOR_CMD=<json>` | arranca este programa en lugar del colector real, como un array JSON `["programa","arg",…]`; el programa habla el protocolo IPC por stdin/stdout. Sirve para que una suite E2E tenga telemetría sin un colector firmado |
//! | `TW_DEV_DATA_DIR=<ruta>` | usa esa carpeta como directorio de datos en vez de `%APPDATA%\com.throttlewatch.desktop`, para que una suite E2E nativa no lea ni escriba los datos reales de quien desarrolla |
#![deny(clippy::unwrap_used, clippy::expect_used)]

#[cfg(any(debug_assertions, feature = "e2e"))]
mod active {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn remaining_write_failures() -> &'static AtomicU32 {
        static REMAINING: OnceLock<AtomicU32> = OnceLock::new();
        REMAINING.get_or_init(|| AtomicU32::new(read_count("TW_DEV_STORAGE_FAIL_WRITES")))
    }

    fn read_count(name: &str) -> u32 {
        std::env::var(name).ok().and_then(|value| value.trim().parse().ok()).unwrap_or(0)
    }

    /// Whether this single write must be reported as failed. Each call consumes one of the
    /// configured failures: with `TW_DEV_STORAGE_FAIL_WRITES=3` the first three writes fail
    /// (the interface warns) and the fourth succeeds (the backlog drains and the warning goes).
    pub fn storage_write_should_fail() -> bool {
        remaining_write_failures()
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                if left == 0 { None } else { Some(left - 1) }
            })
            .is_ok()
    }

    pub fn corrupt_database_requested() -> bool {
        std::env::var("TW_DEV_CORRUPT_DB").is_ok_and(|value| value.trim() == "1")
    }

    /// The program (and arguments) to run in place of the collector, from `TW_DEV_COLLECTOR_CMD`.
    ///
    /// The collector only starts from a manifest signed by a trusted key, and the development
    /// key's private half lives outside the repository on purpose, so a CI runner can never launch
    /// the real one: every scenario that needs live telemetry was unreachable there (the full-disk
    /// scenario failed identically with the manifests hidden locally, 2026-09-26). This replaces the
    /// *launcher*, not the check: the trust boundary is untouched and simply not consulted, in the
    /// only builds where this exists.
    pub fn fake_collector_command() -> Option<(std::path::PathBuf, Vec<String>)> {
        parse_command(&std::env::var("TW_DEV_COLLECTOR_CMD").ok()?)
    }

    /// `["program","arg",…]` as JSON. Anything else — not JSON, not an array of strings, empty, or a
    /// blank program — is no command at all, so a stray value cannot start a half-specified process.
    pub fn parse_command(raw: &str) -> Option<(std::path::PathBuf, Vec<String>)> {
        let parts: Vec<String> = serde_json::from_str(raw).ok()?;
        let (program, args) = parts.split_first()?;
        if program.trim().is_empty() {
            return None;
        }
        Some((std::path::PathBuf::from(program), args.to_vec()))
    }

    /// Where the run must keep its data, when the caller asked for somewhere other than the real
    /// `%APPDATA%\com.throttlewatch.desktop`. Tauri resolves that directory through
    /// `SHGetKnownFolderPath`, which ignores the `APPDATA` variable (measured 2026-09-26: setting
    /// it left the target folder empty), so redirecting it needs an explicit seam like this one.
    pub fn data_dir_override() -> Option<std::path::PathBuf> {
        let raw = std::env::var("TW_DEV_DATA_DIR").ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() { None } else { Some(std::path::PathBuf::from(trimmed)) }
    }
}

#[cfg(not(any(debug_assertions, feature = "e2e")))]
mod active {
    pub fn storage_write_should_fail() -> bool {
        false
    }

    pub fn corrupt_database_requested() -> bool {
        false
    }

    pub fn data_dir_override() -> Option<std::path::PathBuf> {
        None
    }

    pub fn fake_collector_command() -> Option<(std::path::PathBuf, Vec<String>)> {
        None
    }
}

pub use active::{
    corrupt_database_requested, data_dir_override, fake_collector_command,
    storage_write_should_fail,
};

/// The directory this run keeps its data in: `TW_DEV_DATA_DIR` when set (debug/`e2e` only),
/// otherwise the real per-user one Tauri resolves.
///
/// Every caller must go through here rather than `app.path().app_data_dir()`. Mixing the two would
/// be worse than not having the seam at all: `shutdown_services` deletes the `logs` subdirectory
/// when retention is per-session, so a redirected run that resolved that path the other way would
/// delete the developer's real logs.
pub fn resolve_data_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<std::path::PathBuf, tauri::Error> {
    use tauri::Manager;

    match data_dir_override() {
        Some(overridden) => Ok(overridden),
        None => app.path().app_data_dir(),
    }
}

/// Damages the SQLite header of `path` so the next open has to treat the file as corrupt. Does
/// nothing when the fault was not requested, and never touches a path that does not already hold
/// a database — it corrupts what the run would have used, it does not create one.
///
/// Only the 16-byte header is overwritten, on purpose. Truncating the whole file would destroy a
/// developer's real history and would make the export offer meaningless (exporting a stub proves
/// nothing); a real corruption normally leaves most of the file intact, and keeping it that way
/// is what makes "se ofrece exportar la dañada" worth testing.
pub fn corrupt_database_if_requested(path: &std::path::Path) -> bool {
    if !corrupt_database_requested() || !path.is_file() {
        return false;
    }
    // A real SQLite file starts with "SQLite format 3\0"; anything else is rejected as not a
    // database, which is exactly the condition `Storage::open_recovering_corruption` looks for.
    let damaged = overwrite_header(path, b"TW_DEV_CORRUPT!!");

    // The database runs in WAL mode, and a write-ahead log large enough to hold page 1 lets
    // SQLite read a good header straight from it — the main file's damage then goes unnoticed
    // (measured 2026-09-25: with only the main header damaged, the app opened normally). Its
    // 32-byte header is damaged too so the log cannot stand in. Nothing is deleted.
    let wal = path.with_extension("db-wal");
    if wal.is_file() {
        overwrite_header(&wal, b"TW_DEV_CORRUPT_WAL_HEADER_32BYTE");
    }
    damaged
}

fn overwrite_header(path: &std::path::Path, bytes: &[u8]) -> bool {
    use std::io::{Seek, SeekFrom, Write};

    let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(path) else {
        return false;
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return false;
    }
    file.write_all(bytes).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_release_build_can_never_be_faulted() {
        // The point of the cfg split: with neither `debug_assertions` nor `e2e`, both answers are
        // hard-coded false and the environment is never read. This test runs in a debug build, so
        // it asserts the shape that keeps that true: the release module has no env access at all.
        let release_module = include_str!("dev_faults.rs");
        let after_release_cfg = release_module
            .split("#[cfg(not(any(debug_assertions, feature = \"e2e\")))]")
            .nth(1)
            .unwrap_or_default();
        let release_body = after_release_cfg.split("pub use active::").next().unwrap_or_default();
        assert!(
            !release_body.contains("env::var"),
            "the release half of dev_faults must never read the environment"
        );
        // Stronger than the line above and self-maintaining: naming no variable at all means a
        // new `TW_DEV_*` seam cannot be added to the release half without failing here.
        assert!(
            !release_body.contains("TW_DEV"),
            "the release half of dev_faults must not even name a TW_DEV_* variable"
        );
    }

    #[cfg(any(debug_assertions, feature = "e2e"))]
    #[test]
    fn a_json_array_is_a_program_and_its_arguments() {
        let (program, args) = super::active::parse_command(r#"["node","fake.mjs","--fast"]"#)
            .unwrap_or_else(|| panic!("a well-formed command must parse"));
        assert_eq!(program, std::path::PathBuf::from("node"));
        assert_eq!(args, vec!["fake.mjs".to_owned(), "--fast".to_owned()]);
    }

    #[cfg(any(debug_assertions, feature = "e2e"))]
    #[test]
    fn a_command_with_no_arguments_is_still_a_command() {
        let (program, args) = super::active::parse_command(r#"["fake-collector.exe"]"#)
            .unwrap_or_else(|| panic!("a bare program must parse"));
        assert_eq!(program, std::path::PathBuf::from("fake-collector.exe"));
        assert!(args.is_empty());
    }

    #[cfg(any(debug_assertions, feature = "e2e"))]
    #[test]
    fn anything_that_is_not_a_usable_command_is_no_command() {
        for raw in
            ["", "node fake.mjs", "[]", r#"[""]"#, r#"["  "]"#, r#"{"a":1}"#, "[1,2]", "null"]
        {
            assert_eq!(super::active::parse_command(raw), None, "{raw:?}");
        }
    }

    /// The runtime counterpart of `a_release_build_can_never_be_faulted`: that one reads the source,
    /// this one sets every variable and checks the release half really answers "nothing". It only
    /// exists in a build without `debug_assertions`/`e2e`, so `cargo test --release` is what runs it.
    #[cfg(not(any(debug_assertions, feature = "e2e")))]
    #[test]
    fn a_release_build_ignores_every_seam_even_when_the_variables_are_set() {
        // SAFETY: single-threaded test; nothing in the release half caches these.
        unsafe {
            std::env::set_var("TW_DEV_DATA_DIR", "somewhere-else");
            std::env::set_var("TW_DEV_COLLECTOR_CMD", r#"["evil.exe"]"#);
            std::env::set_var("TW_DEV_CORRUPT_DB", "1");
            std::env::set_var("TW_DEV_STORAGE_FAIL_WRITES", "5");
        }
        assert_eq!(super::data_dir_override(), None);
        assert_eq!(super::fake_collector_command(), None);
        assert!(!super::corrupt_database_requested());
        assert!(!super::storage_write_should_fail());
    }

    #[cfg(any(debug_assertions, feature = "e2e"))]
    #[test]
    fn an_absent_or_blank_data_dir_override_is_no_override() {
        // Guards the seam's default: only a non-empty value redirects the data directory, so a
        // stray empty `TW_DEV_DATA_DIR=` cannot send the run to the filesystem root.
        // SAFETY: single-threaded test, and the variable is read on demand rather than cached.
        unsafe { std::env::remove_var("TW_DEV_DATA_DIR") };
        assert_eq!(super::data_dir_override(), None);
        unsafe { std::env::set_var("TW_DEV_DATA_DIR", "   ") };
        assert_eq!(super::data_dir_override(), None);
        unsafe { std::env::set_var("TW_DEV_DATA_DIR", " C:\\tw-e2e ") };
        assert_eq!(super::data_dir_override(), Some(std::path::PathBuf::from("C:\\tw-e2e")));
        unsafe { std::env::remove_var("TW_DEV_DATA_DIR") };
    }

    #[test]
    fn corrupting_is_skipped_when_the_file_does_not_exist() {
        // Guards the "never creates a database" rule: a missing path is simply not corrupted.
        let missing = std::path::Path::new("definitely-not-here-tw-dev-faults.db");
        assert!(!super::corrupt_database_if_requested(missing));
    }
}
