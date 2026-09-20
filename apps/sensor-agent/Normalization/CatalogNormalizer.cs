using System.Globalization;
using System.Text.Json.Serialization;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Normalization;

public sealed record SensorInfo(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("source_id")] string SourceId,
    [property: JsonPropertyName("source_name")] string SourceName,
    [property: JsonPropertyName("metric")] string Metric,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("scope_ref")] string? ScopeRef,
    [property: JsonPropertyName("unit")] string Unit,
    [property: JsonPropertyName("quality")] string Quality);

public sealed record SampleValue(
    [property: JsonPropertyName("sensor_id")] string SensorId,
    [property: JsonPropertyName("number"), JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] double? Number,
    [property: JsonPropertyName("status")] string Status);

/// <summary>
/// The catalog the sidecar publishes with detail "representative": one sensor per magnitude the
/// engine needs (package temperature, total load, package power, highest core clock), chosen with
/// the selection rules of spec.md. The original LibreHardwareMonitor ids stay behind the
/// normalized ones; Rust only sees <c>cpu.package.*</c> ids plus quality and scope.
/// </summary>
public sealed class NormalizedCatalog
{
    private readonly IReadOnlyList<TemperatureSource> temperatures;
    private readonly string? loadRawId;
    private readonly string? powerRawId;
    private readonly IReadOnlyList<string> clockRawIds;

    internal NormalizedCatalog(
        IReadOnlyList<SensorInfo> sensors,
        IReadOnlyList<TemperatureSource> temperatures,
        string? loadRawId,
        string? powerRawId,
        IReadOnlyList<string> clockRawIds)
    {
        Sensors = sensors;
        this.temperatures = temperatures;
        this.loadRawId = loadRawId;
        this.powerRawId = powerRawId;
        this.clockRawIds = clockRawIds;
    }

    public IReadOnlyList<SensorInfo> Sensors { get; }

    /// <summary>Maps one raw sample to the normalized sensors. A missing magnitude stays missing, never zero.</summary>
    public IReadOnlyList<SampleValue> Map(IReadOnlyList<SensorReading> readings)
    {
        var byId = readings.GroupBy(reading => reading.SensorId, StringComparer.Ordinal)
            .ToDictionary(group => group.Key, group => group.First(), StringComparer.Ordinal);
        var values = new List<SampleValue>();
        foreach (var sensor in Sensors)
        {
            values.Add(sensor.Id switch
            {
                CatalogNormalizer.TemperatureId => MapTemperature(byId),
                CatalogNormalizer.LoadId => MapSingle(sensor.Id, loadRawId, byId),
                CatalogNormalizer.PowerId => MapSingle(sensor.Id, powerRawId, byId),
                CatalogNormalizer.ClockId => MapClock(byId),
                _ => new SampleValue(sensor.Id, null, "missing")
            });
        }

        return values;
    }

    private SampleValue MapTemperature(Dictionary<string, SensorReading> byId)
    {
        var readable = temperatures
            .Select(source => (source, reading: byId.GetValueOrDefault(source.RawId)))
            .Where(pair => pair.reading is { Status: "ok", Number: { } })
            .Select(pair => new RawTemperature(
                pair.source.RawId,
                pair.source.Name,
                pair.reading!.Number!.Value,
                pair.source.Scope))
            .ToArray();
        var candidates = readable.Where(temperature => IsMeasurement(temperature.ValueC)).ToArray();
        var chosen = SensorNormalizer.SelectRepresentative(candidates);
        if (chosen is not null)
        {
            return new SampleValue(CatalogNormalizer.TemperatureId, chosen.ValueC, "ok");
        }

        // Something was reported but none of it is a measurement (LibreHardwareMonitorLib gives 0 when it cannot read).
        return new SampleValue(CatalogNormalizer.TemperatureId, null, readable.Length > 0 ? "invalid" : "missing");
    }

    /// <summary>
    /// LibreHardwareMonitorLib reports 0 for a magnitude it cannot read (an AMD CPU without low-level access
    /// gives 0 °C, 0 W and 0 MHz, all "ok"). A running CPU never measures exactly 0 in temperature, package
    /// power or clock, so those zeros are "invalid", never data. Load is exempt: 0 % is a real value.
    /// </summary>
    private static bool IsMeasurement(double value) => value > 0;

    private static SampleValue MapSingle(string id, string? rawId, Dictionary<string, SensorReading> byId)
    {
        if (rawId is null || !byId.TryGetValue(rawId, out var reading))
        {
            return new SampleValue(id, null, "missing");
        }

        if (reading is { Status: "ok", Number: { } number })
        {
            // Only load may be exactly 0.
            return id == CatalogNormalizer.LoadId || IsMeasurement(number)
                ? new SampleValue(id, number, "ok")
                : new SampleValue(id, null, "invalid");
        }

        return new SampleValue(id, null, reading.Status == "ok" ? "invalid" : reading.Status);
    }

    private SampleValue MapClock(Dictionary<string, SensorReading> byId)
    {
        var readable = clockRawIds
            .Select(id => byId.GetValueOrDefault(id))
            .Where(reading => reading is { Status: "ok", Number: { } })
            .Select(reading => (double)reading!.Number!.Value)
            .ToArray();
        var measured = readable.Where(IsMeasurement).ToArray();
        if (measured.Length > 0)
        {
            return new SampleValue(CatalogNormalizer.ClockId, measured.Max(), "ok");
        }

        return new SampleValue(CatalogNormalizer.ClockId, null, readable.Length > 0 ? "invalid" : "missing");
    }

    internal sealed record TemperatureSource(string RawId, string Name, string Scope);
}

public static class CatalogNormalizer
{
    public const string TemperatureId = "cpu.package.temp";
    public const string LoadId = "cpu.package.load";
    public const string PowerId = "cpu.package.power";
    public const string ClockId = "cpu.package.clock";

    public static NormalizedCatalog Build(IReadOnlyList<SensorDescriptor> raw)
    {
        var sensors = new List<SensorInfo>();

        var temperatures = raw
            .Where(descriptor => descriptor.Metric == "temperature")
            .Select(descriptor => (descriptor, scope: InferTemperatureScope(descriptor.SourceName)))
            .Where(pair => pair.scope is not null)
            .Select(pair => new NormalizedCatalog.TemperatureSource(pair.descriptor.Id, pair.descriptor.SourceName, pair.scope!))
            .ToArray();
        if (temperatures.Length > 0)
        {
            var representative = SensorNormalizer.SelectRepresentative(
                temperatures.Select(source => new RawTemperature(source.RawId, source.Name, 0, source.Scope)).ToArray());
            // Tctl without a known offset is a substitute for the die temperature (spec.md, selection rules).
            var quality = representative?.Scope == "tctl" ? "substitute" : "direct";
            sensors.Add(new SensorInfo(TemperatureId, "selection", "Representative CPU temperature", "temperature", "package", null, "celsius", quality));
        }

        var load = raw.FirstOrDefault(descriptor =>
            descriptor.Metric == "load" && descriptor.SourceName.Equals("CPU Total", StringComparison.OrdinalIgnoreCase));
        if (load is not null)
        {
            sensors.Add(new SensorInfo(LoadId, load.Id, load.SourceName, "load", "package", null, "percent", "direct"));
        }

        var power = raw.FirstOrDefault(descriptor =>
            descriptor.Metric == "power" && descriptor.SourceName.Contains("Package", StringComparison.OrdinalIgnoreCase));
        if (power is not null)
        {
            sensors.Add(new SensorInfo(PowerId, power.Id, power.SourceName, "power", "package", null, "watt", "direct"));
        }

        var clocks = raw
            .Where(descriptor => descriptor.Metric == "clock" && IsCoreClock(descriptor.SourceName))
            .Select(descriptor => descriptor.Id)
            .ToArray();
        if (clocks.Length > 0)
        {
            // LibreHardwareMonitor's clock is not the frequency while executing: a substitute of low confidence.
            sensors.Add(new SensorInfo(ClockId, "selection", "Highest core clock", "clock", "package", null, "megahertz", "substitute"));
        }

        return new NormalizedCatalog(sensors, temperatures, load?.Id, power?.Id, clocks);
    }

    /// <summary>Scope classes understood by <see cref="SensorNormalizer.SelectRepresentative"/>; null = not a CPU temperature to use.</summary>
    public static string? InferTemperatureScope(string sourceName)
    {
        var name = sourceName.ToLower(CultureInfo.InvariantCulture);
        if (name.Contains("package", StringComparison.Ordinal))
        {
            return "package";
        }

        if (name.Contains("tctl", StringComparison.Ordinal))
        {
            return "tctl";
        }

        if (name.Contains("tdie", StringComparison.Ordinal))
        {
            return "tdie";
        }

        return name.StartsWith("core", StringComparison.Ordinal) ? "core" : null;
    }

    private static bool IsCoreClock(string sourceName) =>
        sourceName.StartsWith("Core #", StringComparison.OrdinalIgnoreCase)
        || sourceName.StartsWith("CPU Core #", StringComparison.OrdinalIgnoreCase);
}
