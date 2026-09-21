//! The one table mapping a stored `sensor_id` to its `(metric, scope)` (`CatalogNormalizer` in the
//! sidecar, `LimitReasonNormalizer`'s `msr/<flag>` ids, and this host's own `host.*` synthetic
//! clock). `sample_value` stores only the id, never the metric, so anything that reads samples
//! back — export, the CSV columns, replaying a session for [`super::reevaluate`] — needs this to
//! know what a row means. One table, so a sensor the sidecar does not publish yet is `None`
//! everywhere at once, not silently mismatched between two hand-written copies.
#![deny(clippy::unwrap_used, clippy::expect_used)]

/// `(metric, scope)` for a known sensor id, or `None` for one this build does not recognise
/// (an unreleased sensor, or a per-core id with a suffix this table does not list).
pub fn known_sensor(sensor_id: &str) -> Option<(&'static str, &'static str)> {
    match sensor_id {
        "cpu.package.load" => Some(("load", "package")),
        "cpu.package.temp" => Some(("temperature", "package")),
        "cpu.package.power" => Some(("power", "package")),
        // Not published by the sidecar yet (T173): reserved for when the level-A MSR power-limit
        // read lands, under the same `cpu.package.<metric>` convention as the other package ids.
        "cpu.package.power_limit" => Some(("power_limit", "package")),
        "cpu.package.clock" => Some(("active_clock", "package")),
        "host.active_clock" => Some(("active_clock", "host")),
        "host.base_clock" => Some(("base_clock", "host")),
        "msr/thermal_flag" => Some(("thermal_flag", "package")),
        "msr/prochot_flag" => Some(("prochot_flag", "package")),
        "msr/power_flag" => Some(("power_flag", "package")),
        "msr/current_flag" => Some(("current_flag", "package")),
        id => core_sensor(id),
    }
}

/// `cpu.core.<id>.<suffix>` (`CatalogNormalizer.cs`): the id itself is opaque, only the suffix
/// carries meaning.
fn core_sensor(id: &str) -> Option<(&'static str, &'static str)> {
    let rest = id.strip_prefix("cpu.core.")?;
    let (_, suffix) = rest.rsplit_once('.')?;
    match suffix {
        "load" => Some(("load", "core")),
        "temp" => Some(("temperature", "core")),
        "clock" => Some(("active_clock", "core")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::known_sensor;

    #[test]
    fn every_production_package_and_host_id_is_known() {
        for (id, metric, scope) in [
            ("cpu.package.load", "load", "package"),
            ("cpu.package.temp", "temperature", "package"),
            ("cpu.package.power", "power", "package"),
            ("cpu.package.power_limit", "power_limit", "package"),
            ("cpu.package.clock", "active_clock", "package"),
            ("host.active_clock", "active_clock", "host"),
            ("host.base_clock", "base_clock", "host"),
            ("msr/thermal_flag", "thermal_flag", "package"),
            ("msr/prochot_flag", "prochot_flag", "package"),
            ("msr/power_flag", "power_flag", "package"),
            ("msr/current_flag", "current_flag", "package"),
        ] {
            assert_eq!(known_sensor(id), Some((metric, scope)), "{id}");
        }
    }

    #[test]
    fn a_core_id_is_read_from_its_suffix_and_the_core_number_does_not_matter() {
        assert_eq!(known_sensor("cpu.core.1.load"), Some(("load", "core")));
        assert_eq!(known_sensor("cpu.core.p1.temp"), Some(("temperature", "core")));
        assert_eq!(known_sensor("cpu.core.lpe2.clock"), Some(("active_clock", "core")));
        assert_eq!(known_sensor("cpu.core.1.unknown_suffix"), None);
    }

    #[test]
    fn an_unpublished_or_malformed_id_is_none_not_a_guess() {
        for id in ["", "cpu.package.unknown", "cpu.core.", "cpu.core.1", "gpu.temp"] {
            assert_eq!(known_sensor(id), None, "{id}");
        }
    }
}
