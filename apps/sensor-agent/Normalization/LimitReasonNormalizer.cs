namespace ThrottleWatch.SensorAgent.Normalization;

public sealed record IntelLimitReasons(
    bool ThermalFlag,
    bool ProchotFlag,
    bool PowerFlag,
    bool CurrentFlag,
    ulong ClearMask);

public sealed record IntelThermalLimits(
    double? TjMaxC,
    double? TccOffsetC,
    double? EffectiveLimitC,
    double? Pl1W,
    double? Pl2W,
    double? TauS);

public sealed record AmdThermalLimits(string Family, string TableVersion, double ThermalLimitC);

public static class LimitReasonNormalizer
{
    public const uint ThermalBit = 1u << 0;
    public const uint ProchotBit = 1u << 1;
    public const uint PowerBit = 1u << 10;
    public const uint CurrentBit = 1u << 11;
    public const uint ReasonRegister = 0x64F;
    public const uint TemperatureTargetRegister = 0x1A2;
    public const uint PackagePowerLimitRegister = 0x610;

    public static IntelLimitReasons ReadIntelReasons(uint raw)
    {
        const uint allowedMask = ThermalBit | ProchotBit | PowerBit | CurrentBit;
        var flags = raw & allowedMask;
        return new IntelLimitReasons(
            (flags & ThermalBit) != 0,
            (flags & ProchotBit) != 0,
            (flags & PowerBit) != 0,
            (flags & CurrentBit) != 0,
            flags);
    }

    public static IntelThermalLimits ReadIntelLimits(ulong temperatureTarget, ulong packagePowerLimit)
    {
        var tjMax = (temperatureTarget >> 16) & 0xFF;
        // MSR_TEMPERATURE_TARGET bits 27:24 (Intel SDM): a 4-bit field, not the 8 bits this used
        // to mask — a reserved bit 31:28 set would otherwise fabricate a wrong TCC offset (and so
        // a wrong effective thermal limit) for a field never intended to reach that width.
        var tccOffset = (temperatureTarget >> 24) & 0xF;
        var pl1 = DecodePowerLimit(packagePowerLimit & 0x7FFF);
        var pl2 = DecodePowerLimit((packagePowerLimit >> 32) & 0x7FFF);
        var tau = DecodeTau((packagePowerLimit >> 17) & 0x7F);
        var tjMaxC = tjMax is 0 ? null : (double?)tjMax;
        var offsetC = tccOffset is 0 ? null : (double?)tccOffset;
        return new IntelThermalLimits(
            tjMaxC,
            offsetC,
            tjMaxC - offsetC,
            pl1,
            pl2,
            tau);
    }

    public static bool IsAmdThermalFlag(
        string family,
        string tableVersion,
        double thermalPercent,
        double pptPercent,
        double tdcPercent,
        double edcPercent)
    {
        if (!AmdThermalTable.IsAllowed(family, tableVersion))
        {
            return false;
        }
        return thermalPercent >= 99.0
            && pptPercent < 95.0
            && tdcPercent < 95.0
            && edcPercent < 95.0;
    }

    private static double? DecodePowerLimit(ulong encoded)
        => encoded == 0 ? null : encoded / 8.0;

    private static double? DecodeTau(ulong encoded)
        => encoded == 0 ? null : (double)(1UL << (int)(encoded & 0x1F));
}

public static class AmdThermalTable
{
    private static readonly HashSet<string> Allowed = new(StringComparer.OrdinalIgnoreCase)
    {
        "zen4:thermal-limits-v1",
        "zen3:thermal-limits-v1"
    };

    public static bool IsAllowed(string family, string tableVersion)
        => Allowed.Contains($"{family}:{tableVersion}");
}

public interface ILowLevelMsrReader
{
    bool TryRead(uint msrAddress, out ulong value);
    bool TryWrite(uint msrAddress, ulong value);
}

public sealed class IntelLimitDescriptorReader(ILowLevelMsrReader reader)
{
    public IReadOnlyList<RawMetricReading> Read()
    {
        if (!reader.TryRead(LimitReasonNormalizer.ReasonRegister, out var reasons))
        {
            return Array.Empty<RawMetricReading>();
        }

        var normalized = LimitReasonNormalizer.ReadIntelReasons((uint)reasons);
        _ = reader.TryWrite(LimitReasonNormalizer.ReasonRegister, normalized.ClearMask);
        return new[]
        {
            Flag("thermal_flag", normalized.ThermalFlag),
            Flag("prochot_flag", normalized.ProchotFlag),
            Flag("power_flag", normalized.PowerFlag),
            Flag("current_flag", normalized.CurrentFlag)
        };
    }

    private static RawMetricReading Flag(string metric, bool value) => new(
        $"msr/{metric}",
        "msr",
        metric,
        metric,
        "package",
        null,
        value,
        "direct",
        new Dictionary<string, object?>
        {
            ["flag_semantics"] = "log_since_last_sample",
            ["provenance"] = "MSR_CORE_PERF_LIMIT_REASONS"
        });
}
