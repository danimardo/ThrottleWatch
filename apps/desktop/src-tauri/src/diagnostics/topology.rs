use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoreGroup {
    P,
    E,
    Lp,
    Homogeneous,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoreMeasurement {
    pub group: CoreGroup,
    pub active: bool,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupMeasurement {
    pub group: CoreGroup,
    pub active_count: usize,
    pub active_clock_mhz: Option<f64>,
    pub base_clock_mhz: Option<f64>,
    pub below_base: bool,
}

pub fn aggregate_active_groups(measurements: &[CoreMeasurement]) -> Vec<GroupMeasurement> {
    let mut grouped: BTreeMap<CoreGroup, Vec<CoreMeasurement>> = BTreeMap::new();
    for measurement in measurements {
        grouped.entry(measurement.group).or_default().push(*measurement);
    }
    grouped
        .into_iter()
        .map(|(group, values)| {
            let active: Vec<_> = values.iter().filter(|value| value.active).copied().collect();
            let active_clocks: Vec<_> =
                active.iter().filter_map(|value| value.active_clock_mhz).collect();
            let bases: Vec<_> = active.iter().filter_map(|value| value.base_clock_mhz).collect();
            let active_clock_mhz = median(&active_clocks);
            let base_clock_mhz = median(&bases);
            GroupMeasurement {
                group,
                active_count: active.len(),
                active_clock_mhz,
                base_clock_mhz,
                below_base: active_clock_mhz
                    .zip(base_clock_mhz)
                    .is_some_and(|(active, base)| active < base * 0.97),
            }
        })
        .collect()
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[sorted.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::{CoreGroup, CoreMeasurement, aggregate_active_groups};

    #[test]
    fn does_not_average_p_and_e_cores_together() {
        let groups = aggregate_active_groups(&[
            CoreMeasurement {
                group: CoreGroup::P,
                active: true,
                active_clock_mhz: Some(4_000.0),
                base_clock_mhz: Some(3_500.0),
            },
            CoreMeasurement {
                group: CoreGroup::E,
                active: true,
                active_clock_mhz: Some(2_500.0),
                base_clock_mhz: Some(2_000.0),
            },
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].active_clock_mhz, Some(4_000.0));
        assert_eq!(groups[1].active_clock_mhz, Some(2_500.0));
    }
}
