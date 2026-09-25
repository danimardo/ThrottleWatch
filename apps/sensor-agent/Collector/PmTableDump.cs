using System.Text.Json;
using System.Text.Json.Serialization;
using LibreHardwareMonitor.PawnIo;

namespace ThrottleWatch.SensorAgent.Collector;

/// <summary>
/// T028b2: dumps the AMD SMU PM table raw, without interpreting a single field.
///
/// Neither LibreHardwareMonitor 0.9.6 nor this project knows where THM/PPT/TDC/EDC and their
/// limits live inside the table — that map is what T028b2 still needs (see
/// docs/spikes/amd-pm-table.md). So this deliberately decodes nothing: it records the family,
/// the SMU and table versions and every raw word, so the map can be checked against a real
/// capture later, offline, without needing the machine a second time.
/// </summary>
public sealed record PmTableDumpReport(
    [property: JsonPropertyName("cpu_vendor")] string CpuVendor,
    [property: JsonPropertyName("code_name")] long? CodeName,
    [property: JsonPropertyName("smu_version")] string? SmuVersion,
    [property: JsonPropertyName("table_version")] string? TableVersion,
    [property: JsonPropertyName("table_size_bytes")] uint? TableSizeBytes,
    [property: JsonPropertyName("entry_count")] int? EntryCount,
    /// <summary>Raw words exactly as the provider returned them, as hex. Nothing is interpreted.</summary>
    [property: JsonPropertyName("raw_words_hex")] IReadOnlyList<string>? RawWordsHex,
    /// <summary>The low 32 bits of each word read as a float32, the shape the table is known to have.</summary>
    [property: JsonPropertyName("as_float32")] IReadOnlyList<double>? AsFloat32,
    [property: JsonPropertyName("details_code")] string DetailsCode,
    [property: JsonPropertyName("error")] ProbeError? Error = null);

public static class PmTableDump
{
    /// <summary>
    /// Largest known table is about 3.6 KiB (0xE50). A Zen+ desktop answered with 3 144 159 232
    /// bytes on the development machine (2026-09-19, sensor-access.md) — a nonsense size that
    /// would exhaust memory if trusted, so anything past this cap is reported, never allocated.
    /// </summary>
    private const uint MaxPlausibleTableBytes = 64 * 1024;

    private static readonly JsonSerializerOptions JsonOptions =
        new(JsonSerializerDefaults.Web) { WriteIndented = true };

    public static string RunJson() => JsonSerializer.Serialize(Run(), JsonOptions);

    public static PmTableDumpReport Run()
    {
        var vendor = LowLevelAccessProbe.DetectCpuVendor();
        if (vendor != "amd")
        {
            return Failed(vendor, "NOT_AMD");
        }

        if (!PawnIo.IsInstalled)
        {
            return Failed(vendor, "PAWNIO_NOT_INSTALLED");
        }

        var stage = "smu_open";
        try
        {
            // Never through LibreHardwareMonitor.Hardware.Computer: its Close() drops the global
            // PCI bus mutex without clearing it and every later PawnIO call in the process fails.
            var smu = new RyzenSmu();
            try
            {
                stage = "smu_version";
                var smuVersion = smu.GetSmuVersion();
                stage = "code_name";
                var codeName = smu.GetCodeName();
                stage = "resolve_pm_table";
                smu.ResolvePmTable(out var tableVersion, out var tableSize);

                if (tableSize == 0 || tableSize > MaxPlausibleTableBytes)
                {
                    // Zero is what an unprivileged session gets (the SMU version comes back zero
                    // too): no access rather than a broken table. A huge value is the Zen+ case.
                    return new PmTableDumpReport(
                        vendor,
                        codeName,
                        Hex(smuVersion),
                        Hex(tableVersion),
                        tableSize,
                        null,
                        null,
                        null,
                        tableSize == 0 ? "PM_TABLE_UNAVAILABLE" : "PM_TABLE_SIZE_IMPLAUSIBLE");
                }

                stage = "update_pm_table";
                smu.UpdatePmTable();
                stage = "read_pm_table";
                var entryCount = (int)(tableSize / 4);
                var words = smu.ReadPmTable(entryCount);
                if (words is null || words.Length == 0)
                {
                    return new PmTableDumpReport(
                        vendor,
                        codeName,
                        Hex(smuVersion),
                        Hex(tableVersion),
                        tableSize,
                        entryCount,
                        null,
                        null,
                        "PM_TABLE_EMPTY");
                }

                var hex = new List<string>(words.Length);
                var floats = new List<double>(words.Length);
                foreach (var word in words)
                {
                    hex.Add($"0x{word:X16}");
                    floats.Add(BitConverter.Int32BitsToSingle(unchecked((int)(uint)word)));
                }

                return new PmTableDumpReport(
                    vendor,
                    codeName,
                    Hex(smuVersion),
                    Hex(tableVersion),
                    tableSize,
                    words.Length,
                    hex,
                    floats,
                    "PM_TABLE_DUMPED");
            }
            finally
            {
                smu.Close();
            }
        }
        catch (Exception exception)
        {
            return new PmTableDumpReport(
                vendor,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                "PM_TABLE_EXCEPTION",
                new ProbeError(
                    stage,
                    exception.GetType().FullName ?? exception.GetType().Name,
                    exception.Message,
                    $"0x{exception.HResult:X8}"));
        }
    }

    private static string Hex(uint value) => $"0x{value:X8}";

    private static PmTableDumpReport Failed(string vendor, string detailsCode) =>
        new(vendor, null, null, null, null, null, null, null, detailsCode);
}
