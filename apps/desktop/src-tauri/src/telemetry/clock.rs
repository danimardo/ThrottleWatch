use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleQuality {
    Complete,
    Gap,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessorClockReading {
    pub logical_id: u16,
    pub performance_percent: Option<f64>,
    pub processor_frequency_mhz: Option<f64>,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub quality: SampleQuality,
}

pub fn derive_processor_clocks(
    readings: impl IntoIterator<Item = (u16, Option<f64>, Option<f64>, Option<f64>)>,
) -> Vec<ProcessorClockReading> {
    readings
        .into_iter()
        .map(|(logical_id, performance, frequency, base)| ProcessorClockReading {
            logical_id,
            performance_percent: performance,
            processor_frequency_mhz: frequency,
            active_clock_mhz: performance.zip(frequency).and_then(|(p, f)| active_clock_mhz(p, f)),
            base_clock_mhz: base.filter(|value| value.is_finite() && *value >= 0.0),
            quality: if performance
                .zip(frequency)
                .and_then(|(p, f)| active_clock_mhz(p, f))
                .is_some()
            {
                SampleQuality::Complete
            } else {
                SampleQuality::Stale
            },
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct MonotonicClock {
    origin: Instant,
}

impl MonotonicClock {
    pub fn start() -> Self {
        Self { origin: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GapDetector {
    expected_interval: Duration,
    gap_after: Duration,
}

pub fn active_clock_mhz(
    processor_performance_percent: f64,
    processor_frequency_mhz: f64,
) -> Option<f64> {
    if !processor_performance_percent.is_finite()
        || !processor_frequency_mhz.is_finite()
        || !(0.0..=100.0).contains(&processor_performance_percent)
        || processor_frequency_mhz < 0.0
    {
        return None;
    }
    Some(processor_frequency_percent(processor_performance_percent) * processor_frequency_mhz)
}

fn processor_frequency_percent(percent: f64) -> f64 {
    percent / 100.0
}

impl GapDetector {
    pub fn new(expected_interval: Duration) -> Self {
        Self { expected_interval, gap_after: expected_interval.saturating_mul(3) }
    }

    pub fn quality(&self, previous_ms: Option<u64>, current_ms: u64) -> SampleQuality {
        let Some(previous_ms) = previous_ms else {
            return SampleQuality::Complete;
        };
        let delta = Duration::from_millis(current_ms.saturating_sub(previous_ms));
        if delta > self.gap_after {
            SampleQuality::Gap
        } else if delta > self.expected_interval.saturating_mul(2) {
            SampleQuality::Stale
        } else {
            SampleQuality::Complete
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GapDetector, SampleQuality, active_clock_mhz, derive_processor_clocks};
    use std::time::Duration;

    #[test]
    fn classifies_first_sample_and_delays() {
        let detector = GapDetector::new(Duration::from_secs(1));
        assert_eq!(detector.quality(None, 1000), SampleQuality::Complete);
        assert_eq!(detector.quality(Some(1000), 3200), SampleQuality::Stale);
        assert_eq!(detector.quality(Some(1000), 4500), SampleQuality::Gap);
    }

    #[test]
    fn derives_active_clock_from_windows_counters() {
        assert_eq!(active_clock_mhz(50.0, 4_000.0), Some(2_000.0));
        assert_eq!(active_clock_mhz(101.0, 4_000.0), None);
    }

    #[test]
    fn derives_each_logical_processor_without_substituting_zero_for_missing() {
        let readings = derive_processor_clocks([
            (0, Some(50.0), Some(4_000.0), Some(3_500.0)),
            (1, None, Some(4_000.0), Some(3_500.0)),
        ]);
        assert_eq!(readings[0].active_clock_mhz, Some(2_000.0));
        assert_eq!(readings[1].active_clock_mhz, None);
    }
}
