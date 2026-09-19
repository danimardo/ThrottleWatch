using LibreHardwareMonitor.Hardware;

namespace ThrottleWatch.SensorAgent.Collector;

public sealed record SensorDescriptor(
    string Id,
    string SourceId,
    string SourceName,
    string Metric,
    string Unit,
    string Scope = "package",
    string? ScopeRef = null,
    string Quality = "direct",
    IReadOnlyDictionary<string, object?>? Metadata = null);

public sealed record SensorReading(string SensorId, float? Number, string Status);

public interface ICollectorSession
{
    IReadOnlyList<SensorDescriptor> ReadCatalog();

    LowLevelAccessProbeReport ProbeLowLevelAfterCatalog();

    LowLevelAccessProbeReport RecheckCoverage();
}

public sealed class HardwareCollector : ICollectorSession, IDisposable
{
    private readonly Computer computer = new()
    {
        IsCpuEnabled = true,
        IsGpuEnabled = false,
        IsMemoryEnabled = false,
        IsMotherboardEnabled = false,
        IsControllerEnabled = false,
        IsNetworkEnabled = false,
        IsStorageEnabled = false,
        IsBatteryEnabled = false,
        IsPowerMonitorEnabled = false,
        IsPsuEnabled = false
    };

    private bool opened;
    private bool catalogRead;

    public bool IsOpen => opened;

    public void Open()
    {
        if (opened)
        {
            return;
        }

        computer.Open();
        opened = true;
    }

    public IReadOnlyList<SensorDescriptor> ReadCatalog()
    {
        EnsureOpened();
        var catalog = CpuHardware()
            .SelectMany(hardware => hardware.Sensors.Select(sensor =>
                new SensorDescriptor(
                    $"{hardware.Identifier}/{sensor.Name}",
                    hardware.Identifier.ToString(),
                    sensor.Name,
                    MapMetric(sensor.SensorType),
                    MapUnit(sensor.SensorType),
                    MapScope(sensor.Name),
                    ScopeReference(sensor.Name),
                    "direct",
                    new Dictionary<string, object?> { ["provenance"] = "LibreHardwareMonitorLib" })))
            .ToArray();
        catalogRead = true;
        return catalog;
    }

    public LowLevelAccessProbeReport ProbeLowLevelAfterCatalog()
    {
        EnsureCatalogRead();
        return LowLevelAccessProbe.Run();
    }

    public LowLevelAccessProbeReport RecheckCoverage()
    {
        EnsureCatalogRead();
        return LowLevelAccessProbe.Run();
    }

    public IReadOnlyList<SensorReading> ReadSample()
    {
        EnsureOpened();
        var readings = new List<SensorReading>();
        foreach (var hardware in CpuHardware())
        {
            hardware.Update();
            foreach (var sensor in hardware.Sensors)
            {
                var id = $"{hardware.Identifier}/{sensor.Name}";
                var value = sensor.Value;
                readings.Add(value is { } number && IsPhysicallyValid(sensor.SensorType, number)
                    ? new SensorReading(id, number, "ok")
                    : new SensorReading(id, null, "invalid"));
            }
        }
        return readings;
    }

    public void Dispose()
    {
        if (opened)
        {
            computer.Close();
            opened = false;
            catalogRead = false;
        }
    }

    public static bool IsPhysicallyValid(SensorType type, float value)
    {
        if (!float.IsFinite(value))
        {
            return false;
        }
        return type switch
        {
            SensorType.Temperature => value is >= -50 and <= 150,
            SensorType.Load => value is >= 0 and <= 100,
            SensorType.Power => value is >= 0 and <= 2000,
            SensorType.Voltage => value is >= 0 and <= 100,
            SensorType.Clock => value is >= 0 and <= 100_000,
            _ => true
        };
    }

    private IEnumerable<IHardware> CpuHardware() => computer.Hardware.Where(hardware => hardware.HardwareType == HardwareType.Cpu);

    private void EnsureOpened()
    {
        if (!opened)
        {
            throw new InvalidOperationException("collector is not open");
        }
    }

    private void EnsureCatalogRead()
    {
        EnsureOpened();
        if (!catalogRead)
        {
            _ = ReadCatalog();
        }
    }

    private static string MapMetric(SensorType type) => type switch
    {
        SensorType.Temperature => "temperature",
        SensorType.Load => "load",
        SensorType.Clock => "clock",
        SensorType.Power => "power",
        SensorType.Voltage => "voltage",
        _ => "unknown"
    };

    private static string MapUnit(SensorType type) => type switch
    {
        SensorType.Temperature => "celsius",
        SensorType.Load => "percent",
        SensorType.Clock => "megahertz",
        SensorType.Power => "watt",
        SensorType.Voltage => "volt",
        _ => "unknown"
    };

    private static string MapScope(string name)
    {
        var normalized = name.ToLowerInvariant();
        if (normalized.Contains("core", StringComparison.Ordinal)) return "core";
        if (normalized.Contains("thread", StringComparison.Ordinal)) return "thread";
        return "package";
    }

    private static string? ScopeReference(string name)
    {
        var digits = new string(name.Where(char.IsDigit).ToArray());
        return int.TryParse(digits, out var index) ? index.ToString(System.Globalization.CultureInfo.InvariantCulture) : null;
    }
}
