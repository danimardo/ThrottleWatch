//! Active and base clock from the Windows performance counters (`derived`, no elevation, no
//! driver): `Processor Frequency` is the nominal MHz and `% Processor Performance` the share of it
//! the processors actually run at, so `active = nominal × performance / 100` and `base = nominal`.
//! It is what tier B/C machines have when the sidecar publishes no clock (T028c).
#![deny(clippy::unwrap_used, clippy::expect_used)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostClockReading {
    pub active_mhz: f64,
    pub base_mhz: f64,
}

/// Turns the two counter values into a reading; anything that is not a finite positive frequency
/// is rejected instead of being reported as a clock.
pub fn derive(nominal_mhz: f64, performance_percent: f64) -> Option<HostClockReading> {
    let valid = nominal_mhz.is_finite()
        && nominal_mhz > 0.0
        && performance_percent.is_finite()
        && performance_percent > 0.0;
    valid.then_some(HostClockReading {
        active_mhz: nominal_mhz * performance_percent / 100.0,
        base_mhz: nominal_mhz,
    })
}

#[cfg(windows)]
mod windows_counters {
    use super::{HostClockReading, derive};
    use std::sync::{Mutex, OnceLock, PoisonError};
    use windows::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
        PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
    };
    use windows::core::{PCWSTR, w};

    const ERROR_SUCCESS: u32 = 0;
    /// `PDH_CSTATUS_VALID_DATA` and `PDH_CSTATUS_NEW_DATA`.
    const VALID_STATUS: [u32; 2] = [0, 1];

    struct Query {
        query: PDH_HQUERY,
        nominal: PDH_HCOUNTER,
        performance: PDH_HCOUNTER,
    }

    // SAFETY: the PDH handles are plain process-wide identifiers; every use goes through the mutex.
    unsafe impl Send for Query {}

    fn open() -> Option<Query> {
        let mut query = PDH_HQUERY::default();
        let mut nominal = PDH_HCOUNTER::default();
        let mut performance = PDH_HCOUNTER::default();
        // SAFETY: each call receives a valid out pointer and a static, NUL-terminated wide path;
        // the query lives for the whole process, so it is never closed.
        unsafe {
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != ERROR_SUCCESS {
                return None;
            }
            if PdhAddEnglishCounterW(
                query,
                w!(r"\Processor Information(_Total)\Processor Frequency"),
                0,
                &mut nominal,
            ) != ERROR_SUCCESS
                || PdhAddEnglishCounterW(
                    query,
                    w!(r"\Processor Information(_Total)\% Processor Performance"),
                    0,
                    &mut performance,
                ) != ERROR_SUCCESS
            {
                return None;
            }
        }
        Some(Query { query, nominal, performance })
    }

    fn value(counter: PDH_HCOUNTER) -> Option<f64> {
        let mut raw = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: the counter belongs to the open query and `raw` is a valid out structure; the
        // union field is read only after the status says the double is valid.
        unsafe {
            let status = PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut raw);
            (status == ERROR_SUCCESS && VALID_STATUS.contains(&raw.CStatus))
                .then_some(raw.Anonymous.doubleValue)
        }
    }

    pub fn read() -> Option<HostClockReading> {
        static QUERY: OnceLock<Mutex<Option<Query>>> = OnceLock::new();
        let mut guard =
            QUERY.get_or_init(|| Mutex::new(open())).lock().unwrap_or_else(PoisonError::into_inner);
        let query = guard.as_mut()?;
        // SAFETY: the handle comes from `open` and is only used under the mutex.
        if unsafe { PdhCollectQueryData(query.query) } != ERROR_SUCCESS {
            return None;
        }
        // The first collection has no previous sample to rate against: its formatted values are
        // rejected, and the next read (one sampling interval later) succeeds.
        derive(value(query.nominal)?, value(query.performance)?)
    }
}

/// Reads the counters; `None` when they are unavailable or have no previous sample yet.
#[cfg(windows)]
pub fn read() -> Option<HostClockReading> {
    windows_counters::read()
}

#[cfg(not(windows))]
pub fn read() -> Option<HostClockReading> {
    None
}

#[cfg(test)]
mod tests {
    use super::{HostClockReading, derive};

    #[test]
    fn active_is_the_nominal_frequency_scaled_by_the_performance_share() {
        assert_eq!(
            derive(3600.0, 105.0),
            Some(HostClockReading { active_mhz: 3780.0, base_mhz: 3600.0 })
        );
        assert_eq!(
            derive(3600.0, 50.0),
            Some(HostClockReading { active_mhz: 1800.0, base_mhz: 3600.0 })
        );
    }

    #[test]
    fn impossible_counter_values_are_not_reported_as_a_clock() {
        assert_eq!(derive(0.0, 100.0), None);
        assert_eq!(derive(3600.0, 0.0), None);
        assert_eq!(derive(f64::NAN, 100.0), None);
        assert_eq!(derive(3600.0, f64::INFINITY), None);
        assert_eq!(derive(-1.0, 100.0), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_real_counters_deliver_a_plausible_clock_after_two_collections() {
        let _ = super::read();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let reading = super::read().unwrap_or_else(|| panic!("the PDH counters must answer"));
        assert!(reading.base_mhz > 500.0 && reading.base_mhz < 8000.0, "{reading:?}");
        assert!(reading.active_mhz > 100.0 && reading.active_mhz < 10000.0, "{reading:?}");
    }
}
