using LibreHardwareMonitor.PawnIo;
using Microsoft.Extensions.Logging;
using ThrottleWatch.SensorAgent.Logging;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Collector;

/// <summary>One open <see cref="IntelMsr"/> handle behind <see cref="ILowLevelMsrReader"/>, so tests can fake it (constitution XIII).</summary>
internal interface IIntelMsrSession : ILowLevelMsrReader, IDisposable;

internal sealed class PawnIoIntelMsrSession : IIntelMsrSession
{
    private readonly IntelMsr msr = new();

    public bool TryRead(uint msrAddress, out ulong value) => msr.ReadMsr(msrAddress, out value);

    /// <summary>LHM 0.9.6 exposes no MSR write API (confirmed by reflection, docs/spikes/sensor-access.md): the register is never cleared.</summary>
    public bool TryWrite(uint msrAddress, ulong value) => false;

    public void Dispose() => msr.Close();
}

/// <summary>
/// Nivel A (Intel): cablea <see cref="LimitReasonNormalizer"/> y <see cref="IntelLimitDescriptorReader"/>
/// al colector real leyendo <c>MSR_CORE_PERF_LIMIT_REASONS</c> (0x64F), <c>MSR_TEMPERATURE_TARGET</c>
/// (0x1A2) y <c>MSR_PKG_POWER_LIMIT</c> (0x610) mediante PawnIO (contracts/ipc-protocol.md § Muestreo).
/// </summary>
internal static class IntelLimitCatalog
{
    public const string ThermalFlagId = "msr/thermal_flag";
    public const string ProchotFlagId = "msr/prochot_flag";
    public const string PowerFlagId = "msr/power_flag";
    public const string CurrentFlagId = "msr/current_flag";
    public const string PowerLimitId = "cpu.package.power_limit";
    public const string TemperatureTargetId = "msr/temperature_target";
    public const string TemperatureMetadataMetric = "temperature_limit_metadata";
    public const string PowerLimitMetric = "power_limit_direct";

    /// <summary>
    /// One-time read of TjMax/TCC offset for the catalog's temperature descriptor metadata, gated
    /// on the same plausibility check as <see cref="LowLevelAccessProbe"/>: without real access,
    /// PawnIO's <c>ReadMsr</c> still returns <c>true</c> with an all-zero value instead of failing,
    /// so a bare read/write outcome can never be trusted as "access confirmed" — a real TjMax
    /// (50-150 °C) is the only reliable signal. Null (no metadata, no flags) otherwise.
    /// </summary>
    public static IReadOnlyDictionary<string, object?>? ReadTemperatureMetadata(
        Func<bool>? isProviderInstalled = null, Func<IIntelMsrSession>? openSession = null, ILogger? logger = null)
    {
        if (!(isProviderInstalled ?? (() => PawnIo.IsInstalled))())
        {
            return null;
        }

        try
        {
            using var session = (openSession ?? (() => new PawnIoIntelMsrSession()))();
            if (!session.TryRead(LimitReasonNormalizer.TemperatureTargetRegister, out var raw)
                || !LowLevelAccessProbe.IsPlausibleTemperatureTarget(raw))
            {
                return null;
            }

            var limits = LimitReasonNormalizer.ReadIntelLimits(raw, 0);
            if (limits.TjMaxC is not { } tjMax)
            {
                return null;
            }

            var metadata = new Dictionary<string, object?> { ["tjmax_c"] = tjMax };
            if (limits.TccOffsetC is { } offset)
            {
                metadata["tcc_offset_c"] = offset;
            }

            if (limits.EffectiveLimitC is { } effective)
            {
                metadata["thermal_limit_c"] = effective;
            }

            return metadata;
        }
        catch (Exception exception)
        {
            logger?.MsrReadFailed("SENSOR_MSR_TEMPERATURE_TARGET_FAILED", exception.GetType().Name, exception);
            return null;
        }
    }

    /// <summary>
    /// One-time read of <c>Tau</c> (the PL1 averaging window) for the power-limit descriptor's
    /// metadata, mirroring <see cref="ReadTemperatureMetadata"/>. <c>limit_kind</c> (which of PL1/PL2
    /// is effective) is deliberately not published here: it can change every sample, the current
    /// wire <c>SampleValue</c> has no per-sample metadata field to carry it, and nothing on the Rust
    /// side reads it yet (contracts/ipc-protocol.md § Muestreo documents it as forward-looking).
    /// </summary>
    public static IReadOnlyDictionary<string, object?>? ReadPowerLimitMetadata(
        Func<bool>? isProviderInstalled = null, Func<IIntelMsrSession>? openSession = null, ILogger? logger = null)
    {
        if (!(isProviderInstalled ?? (() => PawnIo.IsInstalled))())
        {
            return null;
        }

        try
        {
            using var session = (openSession ?? (() => new PawnIoIntelMsrSession()))();
            if (!session.TryRead(LimitReasonNormalizer.PackagePowerLimitRegister, out var raw))
            {
                return null;
            }

            var limits = LimitReasonNormalizer.ReadIntelLimits(0, raw);
            return limits.TauS is { } tau ? new Dictionary<string, object?> { ["tau_s"] = tau } : null;
        }
        catch (Exception exception)
        {
            logger?.MsrReadFailed("SENSOR_MSR_POWER_LIMIT_FAILED", exception.GetType().Name, exception);
            return null;
        }
    }

    /// <summary>Catalog descriptors declared whenever the vendor is Intel, independent of whether PawnIO is installed right now (a sample reads "missing" when it is not, like every other sensor).</summary>
    public static IEnumerable<SensorDescriptor> Descriptors(
        IReadOnlyDictionary<string, object?>? temperatureMetadata,
        IReadOnlyDictionary<string, object?>? powerLimitMetadata = null)
    {
        if (temperatureMetadata is not null)
        {
            yield return new SensorDescriptor(
                TemperatureTargetId,
                TemperatureTargetId,
                "MSR Temperature Target",
                TemperatureMetadataMetric,
                "celsius",
                "package",
                null,
                "direct",
                temperatureMetadata);
        }

        var flagMetadata = new Dictionary<string, object?>
        {
            ["provenance"] = "MSR_CORE_PERF_LIMIT_REASONS",
            ["flag_semantics"] = "instantaneous"
        };
        yield return Flag(ThermalFlagId, "thermal_flag", "MSR thermal flag", flagMetadata);
        yield return Flag(ProchotFlagId, "prochot_flag", "MSR prochot flag", flagMetadata);
        yield return Flag(PowerFlagId, "power_flag", "MSR power flag", flagMetadata);
        yield return Flag(CurrentFlagId, "current_flag", "MSR current flag", flagMetadata);

        var powerLimitFullMetadata = new Dictionary<string, object?> { ["provenance"] = "MSR_PKG_POWER_LIMIT" };
        if (powerLimitMetadata is not null)
        {
            foreach (var (key, value) in powerLimitMetadata)
            {
                powerLimitFullMetadata[key] = value;
            }
        }

        yield return new SensorDescriptor(
            PowerLimitId,
            PowerLimitId,
            "MSR effective package power limit",
            PowerLimitMetric,
            "watt",
            "package",
            null,
            "direct",
            powerLimitFullMetadata);
    }

    /// <summary>
    /// <paramref name="metric"/> is the sensor's own specific name (<c>thermal_flag</c>, …): it
    /// must reach the wire as-is so <c>telemetry/catalog.rs::known_sensor</c> and
    /// <c>LiveState::flag</c> can find it by (metric, scope); <c>CatalogNormalizer.Build</c>
    /// recognizes these four names to detect a passthrough sensor.
    /// </summary>
    private static SensorDescriptor Flag(string id, string metric, string name, IReadOnlyDictionary<string, object?> metadata) =>
        new(id, id, name, metric, "boolean", "package", null, "direct", metadata);

    /// <summary>
    /// Per-sample read, gated on <paramref name="accessConfirmed"/> (the catalog-time plausibility
    /// check): without it, PawnIO would still answer "ok" with zeros for 0x64F, which is
    /// indistinguishable from a genuine "nothing is limiting right now" — never emitted as data
    /// that was not actually confirmed readable. Always returns exactly one reading per descriptor
    /// <see cref="Descriptors"/> declared for this <paramref name="accessConfirmed"/> state (missing
    /// rather than absent, so a raw catalog id is never left without a matching raw sample id).
    /// </summary>
    public static IReadOnlyList<SensorReading> ReadSample(
        bool accessConfirmed, Func<bool>? isProviderInstalled = null, Func<IIntelMsrSession>? openSession = null, ILogger? logger = null)
    {
        if (!accessConfirmed)
        {
            return MissingReadings(includeTemperatureTarget: false);
        }

        if (!(isProviderInstalled ?? (() => PawnIo.IsInstalled))())
        {
            return MissingReadings(includeTemperatureTarget: true);
        }

        try
        {
            using var session = (openSession ?? (() => new PawnIoIntelMsrSession()))();
            var readings = new List<SensorReading> { new(TemperatureTargetId, null, "missing") };

            var reasons = new IntelLimitDescriptorReader(session).Read();
            if (reasons.Count > 0)
            {
                foreach (var reason in reasons)
                {
                    readings.Add(new SensorReading(reason.SensorId, null, "ok", reason.Boolean));
                }
            }
            else
            {
                readings.Add(new SensorReading(ThermalFlagId, null, "missing"));
                readings.Add(new SensorReading(ProchotFlagId, null, "missing"));
                readings.Add(new SensorReading(PowerFlagId, null, "missing"));
                readings.Add(new SensorReading(CurrentFlagId, null, "missing"));
            }

            if (session.TryRead(LimitReasonNormalizer.PackagePowerLimitRegister, out var powerRaw))
            {
                var limits = LimitReasonNormalizer.ReadIntelLimits(0, powerRaw);
                var effective = EffectivePowerLimit(limits.Pl1W, limits.Pl2W);
                readings.Add(effective is { } watts
                    ? new SensorReading(PowerLimitId, (float)watts, "ok")
                    : new SensorReading(PowerLimitId, null, "missing"));
            }
            else
            {
                readings.Add(new SensorReading(PowerLimitId, null, "missing"));
            }

            return readings;
        }
        catch (Exception exception)
        {
            logger?.MsrReadFailed("SENSOR_MSR_SAMPLE_FAILED", exception.GetType().Name, exception);
            return MissingReadings(includeTemperatureTarget: true);
        }
    }

    private static List<SensorReading> MissingReadings(bool includeTemperatureTarget)
    {
        var readings = new List<SensorReading>();
        if (includeTemperatureTarget)
        {
            readings.Add(new SensorReading(TemperatureTargetId, null, "missing"));
        }

        readings.Add(new SensorReading(ThermalFlagId, null, "missing"));
        readings.Add(new SensorReading(ProchotFlagId, null, "missing"));
        readings.Add(new SensorReading(PowerFlagId, null, "missing"));
        readings.Add(new SensorReading(CurrentFlagId, null, "missing"));
        readings.Add(new SensorReading(PowerLimitId, null, "missing"));
        return readings;
    }

    private static double? EffectivePowerLimit(double? pl1, double? pl2)
    {
        if (pl1 is null)
        {
            return pl2;
        }

        return pl2 is null ? pl1 : Math.Min(pl1.Value, pl2.Value);
    }
}
