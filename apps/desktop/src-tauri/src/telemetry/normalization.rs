#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorQuality {
    Direct,
    Derived,
    Substitute,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorMetadata {
    pub source_id: String,
    pub source_name: String,
    pub scope: String,
    pub quality: SensorQuality,
    pub thermal_limit_c: Option<f64>,
    pub base_clock_mhz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedSensorValue {
    pub sensor_id: String,
    pub metric: String,
    pub number: Option<f64>,
    pub boolean: Option<bool>,
    pub metadata: SensorMetadata,
}

pub fn normalize_number(
    sensor_id: impl Into<String>,
    metric: impl Into<String>,
    number: Option<f64>,
    metadata: SensorMetadata,
) -> NormalizedSensorValue {
    let metric = metric.into();
    let number = number.filter(|value| is_valid(&metric, *value));
    NormalizedSensorValue { sensor_id: sensor_id.into(), metric, number, boolean: None, metadata }
}

pub fn normalize_boolean(
    sensor_id: impl Into<String>,
    metric: impl Into<String>,
    boolean: Option<bool>,
    metadata: SensorMetadata,
) -> NormalizedSensorValue {
    NormalizedSensorValue {
        sensor_id: sensor_id.into(),
        metric: metric.into(),
        number: None,
        boolean,
        metadata,
    }
}

fn is_valid(metric: &str, value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    match metric {
        "temperature" => (-50.0..=150.0).contains(&value),
        "load" => (0.0..=100.0).contains(&value),
        "power" | "power_limit" => (0.0..=2000.0).contains(&value),
        "clock" | "active_clock" | "base_clock" => (0.0..=100_000.0).contains(&value),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{SensorMetadata, SensorQuality, normalize_number};

    fn metadata() -> SensorMetadata {
        SensorMetadata {
            source_id: "source".to_owned(),
            source_name: "sensor".to_owned(),
            scope: "package".to_owned(),
            quality: SensorQuality::Direct,
            thermal_limit_c: None,
            base_clock_mhz: None,
        }
    }

    #[test]
    fn keeps_missing_values_missing_and_rejects_physical_impossibilities() {
        assert_eq!(normalize_number("t", "temperature", None, metadata()).number, None);
        assert_eq!(normalize_number("t", "temperature", Some(200.0), metadata()).number, None);
        assert_eq!(normalize_number("l", "load", Some(80.0), metadata()).number, Some(80.0));
    }
}
