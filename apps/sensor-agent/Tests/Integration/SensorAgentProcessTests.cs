using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.Intrinsics.X86;
using System.Text.Json;
using System.Threading.Tasks;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Integration;

public sealed class SensorAgentProcessTests
{
    // A guest needs both the CPUID "hypervisor present" bit (leaf 1, ECX bit 31) and a
    // virtual SMBIOS product name. The bit alone is also set on bare-metal Windows hosts
    // with virtualization-based security enabled, so it cannot decide on its own.
    // Used to decide whether the degraded-catalog assertions of T-INT-005
    // (`virtualized-no-sensors`) apply; the CI runner is an Azure "Virtual Machine".
    private static readonly string[] VirtualProductNames =
        ["Virtual Machine", "VMware", "VirtualBox", "QEMU", "KVM", "Google Compute Engine"];

    private static bool HypervisorPresent =>
        X86Base.IsSupported
        && (X86Base.CpuId(1, 0).Ecx & (1 << 31)) != 0
        && VirtualProductNames.Any(name => SystemProductName.Contains(name, StringComparison.OrdinalIgnoreCase));

    private static string SystemProductName =>
        Microsoft.Win32.Registry.GetValue(
            @"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\BIOS", "SystemProductName", null) as string ?? string.Empty;

    [Fact]
    [Trait("Category", "Integration")]
    public void ReadsDegradedCatalogWithoutRealSensors()
    {
        using var collector = new HardwareCollector();
        collector.Open();

        var catalog = collector.ReadCatalog();
        var sample = collector.ReadSample();
        var access = collector.ProbeLowLevelAfterCatalog();

        foreach (var descriptor in catalog)
        {
            descriptor.Id.ShouldNotBeNullOrWhiteSpace();
            descriptor.SourceId.ShouldNotBeNullOrWhiteSpace();
            descriptor.Metric.ShouldNotBeNullOrWhiteSpace();
            descriptor.Unit.ShouldNotBeNullOrWhiteSpace();
        }

        sample.Select(reading => reading.SensorId).ShouldBe(catalog.Select(descriptor => descriptor.Id));
        sample.ShouldAllBe(reading => reading.Status == "ok" || reading.Status == "invalid" || reading.Status == "missing");

        var summary = DescribeCatalog(catalog, sample, access, HypervisorPresent);
        TestContext.Current.TestOutputHelper?.WriteLine(summary);
        // Evidence for CHK-L03: CI prints this file after the run so the degraded catalog of the
        // virtualized runner stays visible in the log.
        File.WriteAllText(Path.Combine(AppContext.BaseDirectory, "degraded-catalog.txt"), summary + Environment.NewLine);

        if (!HypervisorPresent)
        {
            return;
        }

        // Virtualized host without sensors: the collector must still open, list whatever the
        // library exposes and report every physical reading as unavailable instead of throwing
        // (the library throws from Update() on the CI runner; the collector maps that to "missing").
        var physical = sample.Where(reading => catalog
            .First(descriptor => descriptor.Id == reading.SensorId).Metric is "temperature" or "power" or "voltage");
        physical.ShouldAllBe(reading => reading.Status == "invalid" || reading.Status == "missing", summary);
        access.State.ShouldNotBe("available", summary);
    }

    [Fact]
    [Trait("Category", "Integration")]
    public async Task StartsRealSidecarWithoutSensorsAndClosesOnEof()
    {
        var sidecarPath = Path.Combine(AppContext.BaseDirectory, "SensorAgent.exe");
        File.Exists(sidecarPath).ShouldBeTrue($"sidecar output is missing: {sidecarPath}");

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = sidecarPath,
                WorkingDirectory = AppContext.BaseDirectory,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            }
        };

        process.Start().ShouldBeTrue();
        var cancellationToken = TestContext.Current.CancellationToken;
        var stderr = process.StandardError.ReadToEndAsync(cancellationToken);
        await process.StandardInput.WriteLineAsync(
            "{\"protocol_version\":1,\"session_nonce\":\"integration-nonce\",\"sequence\":0,\"timestamp_utc\":\"2026-09-19T00:00:00Z\",\"type\":\"hello\",\"payload\":{\"app_version\":\"integration\",\"supported_protocols\":[1]}}");
        await process.StandardInput.FlushAsync(cancellationToken);

        var response = await process.StandardOutput.ReadLineAsync(cancellationToken).AsTask()
            .WaitAsync(TimeSpan.FromSeconds(10), cancellationToken);
        response.ShouldNotBeNullOrWhiteSpace();

        using (var document = JsonDocument.Parse(response!))
        {
            document.RootElement.GetProperty("type").GetString().ShouldBe("hello_ack");
            document.RootElement.GetProperty("session_nonce").GetString().ShouldBe("integration-nonce");
            var state = document.RootElement.GetProperty("payload").GetProperty("low_level_access")
                .GetProperty("state").GetString();
            state.ShouldNotBeNullOrWhiteSpace();
            if (HypervisorPresent)
            {
                state.ShouldNotBe("available", $"a virtualized host must not report low-level access (state={state})");
            }
        }

        await process.StandardInput.DisposeAsync();
        await process.WaitForExitAsync(cancellationToken)
            .WaitAsync(TimeSpan.FromSeconds(10), cancellationToken);
        process.ExitCode.ShouldBe(0);

        // Unhandled exceptions in the sidecar are reported on stderr before exiting.
        var errors = await stderr.WaitAsync(TimeSpan.FromSeconds(5), cancellationToken);
        errors.ShouldNotContain("Exception", customMessage: errors);
    }

    private static string DescribeCatalog(
        System.Collections.Generic.IReadOnlyList<SensorDescriptor> catalog,
        System.Collections.Generic.IReadOnlyList<SensorReading> sample,
        LowLevelAccessProbeReport access,
        bool hypervisor)
    {
        var byMetric = catalog
            .GroupBy(descriptor => descriptor.Metric)
            .Select(group => $"{group.Key}={group.Count()}");
        var okByMetric = sample
            .Where(reading => reading.Status == "ok")
            .GroupBy(reading => catalog.First(descriptor => descriptor.Id == reading.SensorId).Metric)
            .Select(group => $"{group.Key}={group.Count()}");
        return $"hypervisor={hypervisor}; product={SystemProductName}; cpu_vendor={access.CpuVendor}; low_level_access={access.State}/{access.DetailsCode}; "
            + $"catalog[{catalog.Count}]: {string.Join(", ", byMetric)}; ok readings: {string.Join(", ", okByMetric)}";
    }
}
