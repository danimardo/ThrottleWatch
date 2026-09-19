using System.Runtime.Intrinsics.X86;
using System.Text;
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
    [property: JsonPropertyName("error")] ProbeError? Error = null);

public static class LowLevelAccessProbe
{
    private const uint CorePerfLimitReasons = 0x64F;
    private const uint TemperatureTarget = 0x1A2;
    private const uint PackagePowerLimit = 0x610;

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

    public static int DecodeTccOffsetCelsius(ulong value) => (int)((value >> 24) & 0x3F);

    public static bool IsPlausibleTemperatureTarget(ulong value)
    {
        var tjMax = DecodeTjMaxCelsius(value);
        return tjMax is >= 50 and <= 150;
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

                return new LowLevelAccessProbeReport(
                    "intel",
                    state,
                    "pawnio",
                    providerVersion,
                    registers,
                    null,
                    false,
                    detailsCode);
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
