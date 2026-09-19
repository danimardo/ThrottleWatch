namespace ThrottleWatch.SensorAgent.Normalization;

public sealed record RawTemperature(
    string SourceId,
    string SourceName,
    double ValueC,
    string Scope,
    double? TctlOffsetC = null,
    string? ThermalLimitSource = null);

public sealed record NormalizedTemperature(
    string SourceId,
    string SourceName,
    double ValueC,
    string Quality,
    bool IsRepresentative,
    double? ThermalLimitC,
    string SelectionReason);

public sealed record RawMetricReading(
    string SensorId,
    string SourceId,
    string SourceName,
    string Metric,
    string Scope,
    double? Number,
    bool? Boolean,
    string Quality,
    IReadOnlyDictionary<string, object?> Metadata);

public sealed record NormalizedMetricReading(
    string SensorId,
    string SourceId,
    string SourceName,
    string Metric,
    string Scope,
    double? Number,
    bool? Boolean,
    string Quality,
    IReadOnlyDictionary<string, object?> Metadata);

public static class SensorNormalizer
{
    public static IReadOnlyList<NormalizedMetricReading> NormalizeMetrics(
        IEnumerable<RawMetricReading> readings)
    {
        return readings
            .Select(reading => reading with
            {
                Quality = NormalizeQuality(reading.Quality, reading.Number, reading.Boolean),
                Number = NormalizeNumber(reading.Number, reading.Metric),
                Metadata = new Dictionary<string, object?>(reading.Metadata, StringComparer.Ordinal)
            })
            .Select(reading => new NormalizedMetricReading(
                reading.SensorId,
                reading.SourceId,
                reading.SourceName,
                reading.Metric,
                reading.Scope,
                reading.Number,
                reading.Boolean,
                reading.Quality,
                reading.Metadata))
            .ToArray();
    }

    public static IReadOnlyList<NormalizedTemperature> NormalizeTemperatures(IEnumerable<RawTemperature> temperatures)
    {
        var candidates = temperatures
            .Where(static temperature => double.IsFinite(temperature.ValueC))
            .ToArray();
        var representative = SelectRepresentative(candidates);

        return candidates
            .Select(temperature =>
            {
                var selected = ReferenceEquals(temperature, representative);
                var quality = temperature.Scope.Equals("tctl", StringComparison.OrdinalIgnoreCase)
                    && temperature.TctlOffsetC is null
                    ? "substitute"
                    : "direct";
                double? thermalLimit = temperature.TctlOffsetC is { } offset
                    ? temperature.ValueC - offset
                    : null;
                return new NormalizedTemperature(
                    temperature.SourceId,
                    temperature.SourceName,
                    temperature.ValueC,
                    quality,
                    selected,
                    thermalLimit,
                    selected ? SelectionReason(temperature) : "not_selected");
            })
            .ToArray();
    }

    public static RawTemperature? SelectRepresentative(IReadOnlyList<RawTemperature> temperatures)
    {
        var package = temperatures.FirstOrDefault(static temperature =>
            temperature.Scope.Equals("package", StringComparison.OrdinalIgnoreCase)
            || temperature.Scope.Equals("tdie", StringComparison.OrdinalIgnoreCase));
        if (package is not null)
        {
            return package;
        }

        var cores = temperatures
            .Where(static temperature => temperature.Scope.Equals("core", StringComparison.OrdinalIgnoreCase))
            .ToArray();
        if (cores.Length > 0)
        {
            return cores.MaxBy(static temperature => temperature.ValueC);
        }

        var tctlWithOffset = temperatures.FirstOrDefault(static temperature =>
            temperature.Scope.Equals("tctl", StringComparison.OrdinalIgnoreCase)
            && temperature.TctlOffsetC is not null);
        return tctlWithOffset
            ?? temperatures.FirstOrDefault(static temperature => temperature.Scope.Equals("tctl", StringComparison.OrdinalIgnoreCase));
    }

    private static string SelectionReason(RawTemperature temperature) => temperature.Scope.ToLowerInvariant() switch
    {
        "package" or "tdie" => "package_or_tdie",
        "core" => "maximum_core",
        "tctl" when temperature.TctlOffsetC is not null => "tctl_with_offset",
        "tctl" => "tctl_without_offset",
        _ => "fallback"
    };

    private static string NormalizeQuality(string quality, double? number, bool? boolean)
    {
        if (!string.Equals(quality, "direct", StringComparison.OrdinalIgnoreCase)
            && !string.Equals(quality, "derived", StringComparison.OrdinalIgnoreCase)
            && !string.Equals(quality, "substitute", StringComparison.OrdinalIgnoreCase))
        {
            return "unknown";
        }
        return number is null && boolean is null ? "unknown" : quality.ToLowerInvariant();
    }

    private static double? NormalizeNumber(double? number, string metric)
    {
        if (number is null || !double.IsFinite(number.Value))
        {
            return null;
        }
        return metric.ToLowerInvariant() switch
        {
            "load" => number.Value is >= 0 and <= 100 ? number : null,
            "temperature" => number.Value is >= -50 and <= 150 ? number : null,
            "power" or "power_limit" => number.Value is >= 0 and <= 2000 ? number : null,
            "clock" or "active_clock" or "base_clock" => number.Value is >= 0 and <= 100_000 ? number : null,
            _ => number
        };
    }
}
