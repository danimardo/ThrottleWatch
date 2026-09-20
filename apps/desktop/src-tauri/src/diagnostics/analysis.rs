#![deny(clippy::unwrap_used, clippy::expect_used)]

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PointQuality {
    Complete,
    Reduced,
    Missing,
}

impl PointQuality {
    fn rank(self) -> u8 {
        match self {
            Self::Complete => 0,
            Self::Reduced => 1,
            Self::Missing => 2,
        }
    }
    fn worst(self, other: Self) -> Self {
        if self.rank() >= other.rank() { self } else { other }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawPoint {
    pub t_ms: u64,
    pub value: Option<f64>,
    pub quality: PointQuality,
    pub gap: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalysisPoint {
    pub start_ms: u64,
    pub end_ms: u64,
    pub first: Option<f64>,
    pub last: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub average: Option<f64>,
    pub quality: PointQuality,
    pub gap: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalysisTrack {
    pub kind: String,
    pub points: Vec<AnalysisPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventBoundary {
    pub at_ms: u64,
}

pub fn aggregate_track(
    points: &[RawPoint],
    start_ms: u64,
    end_ms: u64,
    target_points: usize,
    boundaries: &[EventBoundary],
) -> Vec<AnalysisPoint> {
    if points.is_empty() || target_points == 0 || end_ms <= start_ms {
        return Vec::new();
    }
    let target = target_points.min(3_000);
    let span = end_ms.saturating_sub(start_ms).max(1);
    let bucket_width = (span as f64 / target as f64).max(1.0);
    let mut output = Vec::new();
    let mut bucket: Vec<RawPoint> = Vec::new();
    let mut bucket_index = usize::MAX;
    let mut previous_t = None;
    for point in
        points.iter().copied().filter(|point| point.t_ms >= start_ms && point.t_ms <= end_ms)
    {
        let index = (((point.t_ms.saturating_sub(start_ms)) as f64 / bucket_width).floor()
            as usize)
            .min(target.saturating_sub(1));
        let crosses_event = previous_t.is_some_and(|previous| {
            boundaries.iter().any(|edge| edge.at_ms > previous && edge.at_ms <= point.t_ms)
        });
        if (!bucket.is_empty() && index != bucket_index) || (!bucket.is_empty() && crosses_event) {
            output.push(finish_bucket(&bucket));
            bucket.clear();
        }
        bucket_index = index;
        bucket.push(point);
        previous_t = Some(point.t_ms);
    }
    if !bucket.is_empty() {
        output.push(finish_bucket(&bucket));
    }
    output
}

fn finish_bucket(points: &[RawPoint]) -> AnalysisPoint {
    let first = points.iter().find_map(|point| point.value);
    let last = points.iter().rev().find_map(|point| point.value);
    let mut min = None;
    let mut max = None;
    let mut sum = 0.0;
    let mut count = 0_u64;
    let mut quality = PointQuality::Complete;
    for point in points {
        quality = quality.worst(point.quality);
        if let Some(value) = point.value {
            min = Some(min.map_or(value, |current: f64| current.min(value)));
            max = Some(max.map_or(value, |current: f64| current.max(value)));
            sum += value;
            count += 1;
        }
    }
    AnalysisPoint {
        start_ms: points.first().map_or(0, |point| point.t_ms),
        end_ms: points.last().map_or(0, |point| point.t_ms),
        first,
        last,
        min,
        max,
        average: (count > 0).then_some(sum / count as f64),
        quality,
        gap: points.iter().any(|point| point.gap || point.value.is_none()),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    fn sample(t_ms: u64, value: f64) -> RawPoint {
        RawPoint { t_ms, value: Some(value), quality: PointQuality::Complete, gap: false }
    }

    #[test]
    fn preserves_extremes_and_average_when_reducing() {
        let result =
            aggregate_track(&[sample(0, 10.0), sample(10, 100.0), sample(20, 20.0)], 0, 30, 1, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].first, Some(10.0));
        assert_eq!(result[0].last, Some(20.0));
        assert_eq!(result[0].min, Some(10.0));
        assert_eq!(result[0].max, Some(100.0));
        assert_eq!(result[0].average, Some(130.0 / 3.0));
    }

    #[test]
    fn keeps_gaps_and_worst_quality_visible() {
        let result = aggregate_track(
            &[
                sample(0, 10.0),
                RawPoint { t_ms: 10, value: None, quality: PointQuality::Missing, gap: true },
            ],
            0,
            20,
            1,
            &[],
        );
        assert!(result[0].gap);
        assert_eq!(result[0].quality, PointQuality::Missing);
        assert_eq!(result[0].last, Some(10.0));
    }

    #[test]
    fn event_edges_split_buckets_even_when_resolution_is_low() {
        let result = aggregate_track(
            &[sample(0, 1.0), sample(50, 2.0), sample(100, 3.0)],
            0,
            100,
            1,
            &[EventBoundary { at_ms: 50 }],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].end_ms, 0);
        assert_eq!(result[1].start_ms, 50);
    }

    #[test]
    fn reduction_is_bounded_to_three_thousand_points() {
        let points: Vec<_> = (0..5000).map(|value| sample(value, value as f64)).collect();
        let result = aggregate_track(&points, 0, 5000, 10_000, &[]);
        assert!(result.len() <= 3_000);
    }

    proptest! {
        #[test]
        fn aggregation_preserves_extremes_and_stays_within_the_target(
            values in prop::collection::vec(-10_000.0_f64..10_000.0, 1..100),
            target in 1_usize..20,
        ) {
            let points: Vec<_> = values
                .iter()
                .enumerate()
                .map(|(index, value)| sample(index as u64 * 10, *value))
                .collect();
            let result = aggregate_track(&points, 0, 1_000, target, &[]);
            prop_assert!(result.len() <= target.min(3_000));
            let expected_min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let expected_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let actual_min = result.iter().filter_map(|point| point.min).fold(f64::INFINITY, f64::min);
            let actual_max = result.iter().filter_map(|point| point.max).fold(f64::NEG_INFINITY, f64::max);
            prop_assert_eq!(actual_min, expected_min);
            prop_assert_eq!(actual_max, expected_max);
        }
    }
}
