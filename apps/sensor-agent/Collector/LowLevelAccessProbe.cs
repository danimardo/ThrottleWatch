using System.Diagnostics;
using System.Runtime.Intrinsics.X86;
using System.Text;
using System.Threading;
using System.Text.Json;
using System.Text.Json.Serialization;
using LibreHardwareMonitor.PawnIo;

namespace ThrottleWatch.SensorAgent.Collector;

public sealed record RegisterProbe(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("address")] string Address,
    [property: JsonPropertyName("readable")] bool Readable,
    [property: JsonPropertyName("value_hex")] string? ValueHex,
    [property: JsonPropertyName("details_code")] string? DetailsCode);

public sealed record ProbeError(
    [property: JsonPropertyName("stage")] string Stage,
    [property: JsonPropertyName("exception_type")] string ExceptionType,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("hresult")] string HResult);

public sealed record LowLevelAccessProbeReport(
    [property: JsonPropertyName("cpu_vendor")] string CpuVendor,
    [property: JsonPropertyName("state")] string State,
    [property: JsonPropertyName("provider")] string? Provider,
    [property: JsonPropertyName("provider_version")] string? ProviderVersion,
    [property: JsonPropertyName("registers")] IReadOnlyList<RegisterProbe> Registers,
    [property: JsonPropertyName("smu_version")] string? SmuVersion,
    [property: JsonPropertyName("log_clear_supported")] bool LogClearSupported,
    [property: JsonPropertyName("details_code")] string? DetailsCode,
    [property: JsonPropertyName("error")] ProbeError? Error = null,
    // T019: contrasts % Processor Performance (PDH, host side) against the CPU's own
    // APERF/MPERF ratio, read directly in the sidecar. Null when the MSR could not be
    // sampled twice (no access, or MPERF did not advance in the sampling window).
    [property: JsonPropertyName("aperf_mperf_ratio")] double? AperfMperfRatio = null,
    [property: JsonPropertyName("aperf_mperf_window_ms")] int? AperfMperfWindowMs = null);

public static class LowLevelAccessProbe
{
    private const uint CorePerfLimitReasons = 0x64F;
    private const uint TemperatureTarget = 0x1A2;
    private const uint PackagePowerLimit = 0x610;
    private const uint Ia32Aperf = 0xE7;
    private const uint Ia32Mperf = 0xE8;
    private const int AperfMperfSampleWindowMs = 200;

    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = true
    };

    public static string RunJson()
    {
        return ToJson(Run());
    }

    public static string ToJson(LowLevelAccessProbeReport report)
    {
        return JsonSerializer.Serialize(report, JsonOptions);
    }

    public static LowLevelAccessProbeReport Run()
    {
        var vendor = DetectCpuVendor();
        if (!PawnIo.IsInstalled)
        {
            return Missing(vendor, "PAWNIO_NOT_INSTALLED");
        }

        var providerVersion = PawnIo.Version?.ToString();
        return vendor switch
        {
            "intel" => ProbeIntel(providerVersion),
            "amd" => ProbeAmd(providerVersion),
            _ => new LowLevelAccessProbeReport(
                vendor,
                "unknown",
                "pawnio",
                providerVersion,
                [],
                null,
                false,
                "CPU_VENDOR_UNKNOWN")
        };
    }

    /// <summary>
    /// Reads the vendor string from CPUID leaf 0. Deliberately avoids
    /// <c>LibreHardwareMonitor.Hardware.Computer</c>: its <c>Close()</c> disposes the global
    /// PCI bus mutex without clearing it, and every later PawnIO call in the same process
    /// (RyzenSmu, IntelMsr) then fails with "Timeout waiting for PCI bus mutex".
    /// </summary>
    public static string DetectCpuVendor()
    {
        if (!X86Base.IsSupported)
        {
            return "unknown";
        }

        var (_, ebx, ecx, edx) = X86Base.CpuId(0, 0);
        var bytes = new byte[12];
        BitConverter.TryWriteBytes(bytes.AsSpan(0, 4), ebx);
        BitConverter.TryWriteBytes(bytes.AsSpan(4, 4), edx);
        BitConverter.TryWriteBytes(bytes.AsSpan(8, 4), ecx);
        return ClassifyCpuVendor(Encoding.ASCII.GetString(bytes));
    }

    public static string ClassifyCpuVendor(string vendorId)
    {
        return vendorId switch
        {
            "GenuineIntel" => "intel",
            "AuthenticAMD" => "amd",
            _ => "unknown"
        };
    }

    public static int DecodeTjMaxCelsius(ulong value) => (int)((value >> 16) & 0xFF);

    // MSR_TEMPERATURE_TARGET bits 27:24 (Intel SDM): a 4-bit field; matches LimitReasonNormalizer.ReadIntelLimits.
    public static int DecodeTccOffsetCelsius(ulong value) => (int)((value >> 24) & 0xF);

    public static bool IsPlausibleTemperatureTarget(ulong value)
    {
        var tjMax = DecodeTjMaxCelsius(value);
        return tjMax is >= 50 and <= 150;
    }

    /// <summary>
    /// APERF/MPERF ratio for the sampling window, contrasted in the spike against the host's
    /// PDH-derived <c>% Processor Performance</c> (research.md § "Riesgos abiertos"). Null when
    /// MPERF (the fixed-rate reference clock) did not advance: no valid window to measure.
    /// </summary>
    public static double? ComputeAperfMperfRatio(ulong aperf0, ulong mperf0, ulong aperf1, ulong mperf1)
    {
        if (mperf1 <= mperf0)
        {
            return null;
        }

        var aperfDelta = aperf1 - aperf0;
        var mperfDelta = mperf1 - mperf0;
        return aperfDelta / (double)mperfDelta;
    }

    private static LowLevelAccessProbeReport ProbeIntel(string? providerVersion)
    {
        try
        {
            var msr = new IntelMsr();
            try
            {
                var registers = new[]
                {
                    ReadRegister(msr, "core_perf_limit_reasons", CorePerfLimitReasons),
                    ReadRegister(msr, "temperature_target", TemperatureTarget),
                    ReadRegister(msr, "package_power_limit", PackagePowerLimit)
                };
                var thermalTarget = registers.Single(register => register.Address == "0x1A2");
                var allReadable = registers.All(register => register.Readable);
                var plausibleThermalTarget = thermalTarget.ValueHex is not null
                    && IsPlausibleTemperatureTarget(Convert.ToUInt64(thermalTarget.ValueHex[2..], 16));
                var state = allReadable && plausibleThermalTarget ? "available" : "denied";
                var detailsCode = state == "available"
                    ? "MSR_READ_OK_LOG_CLEAR_UNSUPPORTED"
                    : "MSR_READ_FAILED";
                var aperfMperfRatio = SampleAperfMperfRatio(msr);

                return new LowLevelAccessProbeReport(
                    "intel",
                    state,
                    "pawnio",
                    providerVersion,
                    registers,
                    null,
                    false,
                    detailsCode,
                    AperfMperfRatio: aperfMperfRatio,
                    AperfMperfWindowMs: aperfMperfRatio is null ? null : AperfMperfSampleWindowMs);
            }
            finally
            {
                msr.Close();
            }
        }
        catch (Exception exception)
        {
            return new LowLevelAccessProbeReport(
                "intel",
                "error",
                "pawnio",
                providerVersion,
                [],
                null,
                false,
                "MSR_PROBE_EXCEPTION",
                Describe("msr_read", exception));
        }
    }

    private static LowLevelAccessProbeReport ProbeAmd(string? providerVersion)
    {
        var stage = "smu_open";
        try
        {
            var smu = new RyzenSmu();
            try
            {
                stage = "smu_version";
                var version = smu.GetSmuVersion();
                stage = "pm_table";
                smu.ResolvePmTable(out _, out _);
                return new LowLevelAccessProbeReport(
                    "amd",
                    version == 0 ? "denied" : "available",
                    "pawnio",
                    providerVersion,
                    [],
                    $"0x{version:X8}",
                    false,
                    version == 0 ? "SMU_VERSION_ZERO" : "AMD_PM_TABLE_REQUIRES_ALLOWLIST");
            }
            finally
            {
                smu.Close();
            }
        }
        catch (Exception exception)
        {
            return new LowLevelAccessProbeReport(
                "amd",
                "error",
                "pawnio",
                providerVersion,
                [],
                null,
                false,
                "AMD_SMU_READ_FAILED",
                Describe(stage, exception));
        }
    }

    private static ProbeError Describe(string stage, Exception exception)
    {
        return new ProbeError(
            stage,
            exception.GetType().FullName ?? exception.GetType().Name,
            exception.Message,
            $"0x{exception.HResult:X8}");
    }

    // APERF/MPERF are per logical processor. Without pinning, the OS scheduler can migrate this
    // thread between the two reads, mixing two independent counters and always yielding a
    // meaningless (or negative) delta - observed in practice on this 22-thread hybrid CPU under
    // load. Process affinity is restored afterward regardless of outcome.
    private static double? SampleAperfMperfRatio(IntelMsr msr)
    {
        var process = Process.GetCurrentProcess();
        var originalAffinity = process.ProcessorAffinity;
        try
        {
            process.ProcessorAffinity = 1;

            var before = ReadAperfMperf(msr);
            if (before is null)
            {
                return null;
            }

            Thread.Sleep(AperfMperfSampleWindowMs);

            var after = ReadAperfMperf(msr);
            if (after is null)
            {
                return null;
            }

            var (aperf0, mperf0) = before.Value;
            var (aperf1, mperf1) = after.Value;
            return ComputeAperfMperfRatio(aperf0, mperf0, aperf1, mperf1);
        }
        finally
        {
            process.ProcessorAffinity = originalAffinity;
        }
    }

    private static (ulong Aperf, ulong Mperf)? ReadAperfMperf(IntelMsr msr)
    {
        if (!msr.ReadMsr(Ia32Aperf, out ulong aperf) || !msr.ReadMsr(Ia32Mperf, out ulong mperf))
        {
            return null;
        }

        return (aperf, mperf);
    }

    private static RegisterProbe ReadRegister(IntelMsr msr, string name, uint address)
    {
        if (!msr.ReadMsr(address, out ulong value))
        {
            return new RegisterProbe(name, $"0x{address:X}", false, null, "MSR_READ_FAILED");
        }

        return new RegisterProbe(name, $"0x{address:X}", true, $"0x{value:X16}", null);
    }

    private static LowLevelAccessProbeReport Missing(string vendor, string detailsCode)
    {
        return new LowLevelAccessProbeReport(
            vendor,
            "missing",
            "pawnio",
            null,
            [],
            null,
            false,
            detailsCode);
    }
}
