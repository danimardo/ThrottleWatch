//! Free disk space of the volume that holds a given directory, read directly via
//! `GetDiskFreeSpaceExW` (kernel32) — the same style of raw FFI as
//! `telemetry::power_context`, kept out of the `windows` crate's larger surface for one call.
//! The guided-test preflight (T167) is the only caller: the test writes real samples for minutes,
//! so it must not start when the volume plausibly cannot hold them.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

/// `None` when the call fails (missing path, no such volume, …); the preflight then treats disk
/// space as unavailable rather than fabricating a number (constitution: absent data never becomes
/// zero — that applies to a safety gate exactly as it does to a sensor reading).
#[cfg(windows)]
pub fn free_disk_mb(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut free_bytes_available_to_caller: u64 = 0;
    // SAFETY: `wide` is a valid null-terminated UTF-16 string kept alive for the whole call, and
    // the one out-parameter we pass points at a local `u64` the Windows API is documented to fill
    // in place; the two parameters we do not need are null, which the API accepts.
    let ok = unsafe {
        get_disk_free_space_ex_w(
            wide.as_ptr(),
            &mut free_bytes_available_to_caller,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0;
    ok.then_some(free_bytes_available_to_caller / (1024 * 1024))
}

#[cfg(not(windows))]
pub fn free_disk_mb(_path: &Path) -> Option<u64> {
    None
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "GetDiskFreeSpaceExW"]
    fn get_disk_free_space_ex_w(
        directory_name: *const u16,
        free_bytes_available_to_caller: *mut u64,
        total_number_of_bytes: *mut u64,
        total_number_of_free_bytes: *mut u64,
    ) -> i32;
}

#[cfg(all(test, windows))]
mod tests {
    use super::free_disk_mb;
    use std::path::Path;

    #[test]
    fn reads_a_plausible_free_space_for_the_current_directory() {
        let mb = free_disk_mb(Path::new("."))
            .unwrap_or_else(|| panic!("the current volume must be readable"));
        assert!(mb > 0, "a real, non-full volume always reports some free space");
    }

    #[test]
    fn a_drive_that_does_not_exist_reports_none() {
        assert_eq!(free_disk_mb(Path::new("Z:\\this-drive-should-not-exist-tw")), None);
    }
}
