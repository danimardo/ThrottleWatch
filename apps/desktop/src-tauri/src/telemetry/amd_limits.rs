#![deny(clippy::unwrap_used, clippy::expect_used)]

//! The thermal limit of an AMD processor, from a versioned table (FR-076): AMD publishes no register
//! the collector could read it from, so it comes from the model AMD lists it for. A model that is not
//! in the table has no known limit; nothing is guessed from the generation.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE_JSON: &str = include_str!("../../resources/amd-thermal-limits.json");

#[derive(Debug, Deserialize)]
struct Table {
    limits_c: BTreeMap<String, f64>,
}

fn table() -> Option<&'static Table> {
    static TABLE: OnceLock<Option<Table>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(TABLE_JSON).ok()).as_ref()
}

/// The model part of a Ryzen name: `"AMD Ryzen 5 2600X Six-Core Processor"` → `"2600X"`.
fn model_token(display_name: &str) -> Option<&str> {
    let mut words = display_name.split_whitespace();
    words.find(|word| word.eq_ignore_ascii_case("ryzen"))?;
    let _tier = words.next()?;
    words.next()
}

/// The maximum operating temperature AMD lists for this processor, if it is one the table knows.
pub fn tjmax_c(display_name: &str) -> Option<f64> {
    let model = model_token(display_name)?;
    table()?.limits_c.get(&model.to_ascii_uppercase()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_ryzen_model_gets_the_limit_amd_lists() {
        assert_eq!(tjmax_c("AMD Ryzen 5 2600X Six-Core Processor"), Some(95.0));
        assert_eq!(tjmax_c("AMD Ryzen 7 7800X3D 8-Core Processor"), Some(89.0));
        assert_eq!(tjmax_c("AMD Ryzen 5 3600XT 6-Core Processor"), Some(95.0));
        assert_eq!(tjmax_c("amd ryzen 5 2600x six-core processor"), Some(95.0));
        assert_eq!(tjmax_c("AMD Ryzen 5 5600X 6-Core Processor"), Some(95.0));
        assert_eq!(tjmax_c("AMD Ryzen 5 5600 6-Core Processor"), Some(90.0));
        assert_eq!(tjmax_c("AMD Ryzen 9 7950X3D 16-Core Processor"), Some(89.0));
        assert_eq!(tjmax_c("AMD Ryzen 7 8700G w/ Radeon 780M Graphics"), Some(95.0));
    }

    #[test]
    fn a_model_with_a_tctl_offset_or_not_in_the_table_has_no_limit() {
        for name in [
            "AMD Ryzen 7 1800X Eight-Core Processor",
            "AMD Ryzen 7 2700X Eight-Core Processor",
            "AMD Ryzen Threadripper 2990WX 32-Core Processor",
            "AMD Ryzen 7 5800U with Radeon Graphics",
            "AMD Ryzen 5 1600 Six-Core Processor",
            "AMD Ryzen 3 1200 Quad-Core Processor",
            "AMD Ryzen 7 2700E Eight-Core Processor",
            "AMD Ryzen 9 3900 12-Core Processor",
            "AMD Ryzen 3 3300X 4-Core Processor",
            "AMD Ryzen 5 PRO 5650G with Radeon Graphics",
            "AMD Athlon 3000G",
            "Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz",
            "AMD Ryzen 5",
            "",
        ] {
            assert_eq!(tjmax_c(name), None, "{name}");
        }
    }

    #[test]
    fn the_table_parses_and_every_limit_is_physically_plausible() {
        let table = table().unwrap_or_else(|| panic!("the table must parse"));
        assert!(!table.limits_c.is_empty());
        for (model, limit) in &table.limits_c {
            assert!((80.0..=105.0).contains(limit), "{model}: {limit}");
            assert_eq!(model, &model.to_ascii_uppercase(), "{model} must be upper case");
        }
    }
}
