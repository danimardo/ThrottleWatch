using LibreHardwareMonitor.Hardware;
using Microsoft.Extensions.Logging;
using ThrottleWatch.SensorAgent.Logging;

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

public sealed record SensorReading(string SensorId, float? Number, string Status, bool? Boolean = null);

/// <summary>What the protocol server needs from the hardware layer (a fake in tests).</summary>
public interface ISensorSource : ICollectorSession
{
    string CpuDisplayName { get; }

    IReadOnlyList<SensorReading> ReadSample();
}

public interface ICollectorSession
{
    IReadOnlyList<SensorDescriptor> ReadCatalog();

    LowLevelAccessProbeReport ProbeLowLevelAfterCatalog();

    LowLevelAccessProbeReport RecheckCoverage();
}

/// <summary>
/// Seam over LibreHardwareMonitor's <see cref="Computer"/> so the collector can be tested
/// without hardware (constitution XIII): the production source wraps the real computer;
/// tests provide fake <see cref="IHardware"/> instances.
/// </summary>
public interface IHardwareSource : IDisposable
{
    void Open();

    void Close();

    IEnumerable<IHardware> CpuHardware();
}

internal sealed class ComputerHardwareSource : IHardwareSource
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

    public void Open() => computer.Open();

    public void Close() => computer.Close();

    public IEnumerable<IHardware> CpuHardware() => computer.Hardware.Where(hardware => hardware.HardwareType == HardwareType.Cpu);

    public void Dispose()
    {
    }
}

public sealed class HardwareCollector : ISensorSource, IDisposable
{
    private readonly IHardwareSource source;
    private readonly ILogger logger;
    private readonly HashSet<string> failedUpdates = [];
    private readonly Func<bool> isVirtualized;
    private readonly Func<string> detectVendor;
    private bool opened;
    private bool catalogRead;
    private bool virtualizedHost;
    private bool openFailed;
    private string vendor = "unknown";
    private bool intelMsrAccessConfirmed;

    public HardwareCollector()
        : this(new ComputerHardwareSource(), Log.Create("sensor-agent.collector"), HostVirtualization.IsVirtualized)
    {
    }

    /// <param name="detectVendor">Seam over <see cref="LowLevelAccessProbe.DetectCpuVendor"/> (constitution XIII): tests fix the vendor instead of depending on the real CPU running the suite.</param>
    internal HardwareCollector(IHardwareSource source, ILogger logger, Func<bool>? isVirtualized = null, Func<string>? detectVendor = null)
    {
        this.source = source;
        this.logger = logger;
        this.isVirtualized = isVirtualized ?? (() => false);
        this.detectVendor = detectVendor ?? LowLevelAccessProbe.DetectCpuVendor;
    }

    public bool IsOpen => opened;

    public string CpuDisplayName
    {
        get
        {
            if (openFailed)
            {
                return "CPU";
            }

            var name = source.CpuHardware().FirstOrDefault()?.Name;
            return string.IsNullOrWhiteSpace(name) ? "CPU" : name;
        }
    }

    public void Open()
    {
        if (opened)
        {
            return;
        }

        try
        {
            source.Open();
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            // LibreHardwareMonitorLib throws from Computer.Open() on some virtualized hosts
            // (NullReferenceException in GenericCpu.EstimateTimeStampCounterFrequency). Its state
            // is undefined afterwards, so the collector runs with no hardware instead of dying.
            openFailed = true;
            logger.HardwareOpenFailed("SENSOR_OPEN_FAILED", exception.GetType().Name, exception);
        }

        opened = true;
        virtualizedHost = isVirtualized();
        if (virtualizedHost)
        {
            logger.HostVirtualized("HOST_VIRTUALIZED");
        }

        vendor = detectVendor();
    }

    public IReadOnlyList<SensorDescriptor> ReadCatalog()
    {
        EnsureOpened();
        if (openFailed)
        {
            catalogRead = true;
            return [];
        }

        var catalog = source.CpuHardware()
            .SelectMany(hardware => hardware.Sensors.Where(sensor => !IsHiddenOnThisHost(sensor.SensorType)).Select(sensor =>
                new SensorDescriptor(
                    SensorId(sensor),
                    hardware.Identifier.ToString(),
                    sensor.Name,
                    MapMetric(sensor.SensorType),
                    MapUnit(sensor.SensorType),
                    MapScope(sensor.Name),
                    ScopeReference(sensor.Name),
                    "direct",
                    new Dictionary<string, object?> { ["provenance"] = "LibreHardwareMonitorLib" })))
            .ToArray();

        if (vendor == "intel")
        {
            // Safe alongside the already-open Computer (T162): IntelMsr never touches LHM's PCI bus mutex.
            var temperatureMetadata = IntelLimitCatalog.ReadTemperatureMetadata(logger: logger);
            intelMsrAccessConfirmed = temperatureMetadata is not null;
            var powerLimitMetadata = intelMsrAccessConfirmed ? IntelLimitCatalog.ReadPowerLimitMetadata(logger: logger) : null;
            catalog = catalog.Concat(IntelLimitCatalog.Descriptors(temperatureMetadata, powerLimitMetadata)).ToArray();
        }

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
        if (openFailed)
        {
            return readings;
        }

        foreach (var hardware in source.CpuHardware())
        {
            // A host without readable sensors (virtualized CI runner, no low-level access)
            // makes the library throw from inside Update(); the sidecar must keep running
            // and report the readings as missing instead of dying (T-INT-005).
            var updated = TryUpdate(hardware);
            foreach (var sensor in hardware.Sensors.Where(sensor => !IsHiddenOnThisHost(sensor.SensorType)))
            {
                var id = SensorId(sensor);
                if (!updated)
                {
                    readings.Add(new SensorReading(id, null, "missing"));
                    continue;
                }
                var value = sensor.Value;
                readings.Add(value is { } number && IsPhysicallyValid(sensor.SensorType, number)
                    ? new SensorReading(id, number, "ok")
                    : new SensorReading(id, null, "invalid"));
            }
        }

        if (vendor == "intel")
        {
            readings.AddRange(IntelLimitCatalog.ReadSample(intelMsrAccessConfirmed, logger: logger));
        }

        return readings;
    }

    public void Dispose()
    {
        if (opened)
        {
            if (!openFailed)
            {
                source.Close();
            }

            source.Dispose();
            opened = false;
            catalogRead = false;
        }
    }

    /// <summary>
    /// LibreHardwareMonitor's own sensor identifier (for example /amdcpu/0/clock/1) is unique per sensor;
    /// hardware id plus name is not: "Core #1" exists as a clock and as a multiplier factor.
    /// </summary>
    private static string SensorId(ISensor sensor) => sensor.Identifier.ToString();

    /// <summary>Guests report physical sensors from virtual MSRs; those are not measurements (see HostVirtualization).</summary>
    private bool IsHiddenOnThisHost(SensorType type) =>
        virtualizedHost && type is SensorType.Temperature or SensorType.Power or SensorType.Voltage;

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

    private bool TryUpdate(IHardware hardware)
    {
        try
        {
            hardware.Update();
            failedUpdates.Remove(hardware.Identifier.ToString());
            return true;
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            // Logged once per hardware: the failure repeats on every sample.
            if (failedUpdates.Add(hardware.Identifier.ToString()))
            {
                logger.HardwareUpdateFailed("SENSOR_UPDATE_FAILED", hardware.Identifier.ToString(), exception.GetType().Name, exception);
            }
            return false;
        }
    }

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
