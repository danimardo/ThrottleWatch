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
}

#[cfg(not(any(debug_assertions, feature = "e2e")))]
mod active {
    pub fn storage_write_should_fail() -> bool {
        false
    }

    pub fn corrupt_database_requested() -> bool {
        false
    }
}

pub use active::{corrupt_database_requested, storage_write_should_fail};

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
    }

    #[test]
    fn corrupting_is_skipped_when_the_file_does_not_exist() {
        // Guards the "never creates a database" rule: a missing path is simply not corrupted.
        let missing = std::path::Path::new("definitely-not-here-tw-dev-faults.db");
        assert!(!super::corrupt_database_if_requested(missing));
    }
}
