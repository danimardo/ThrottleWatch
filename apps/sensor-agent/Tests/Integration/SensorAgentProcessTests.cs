using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Integration;

public sealed class SensorAgentProcessTests
{
    // The collector hides temperature, power and voltage on a guest (HostVirtualization);
    // T-INT-005 checks that on the virtualized CI runner, whatever CPU vendor it lands on.
    private static bool HypervisorPresent => HostVirtualization.IsVirtualized();

    // Evidence only (printed in the CI log); the decision above uses the production classifier.
    private static string SystemProductName =>
        Microsoft.Win32.Registry.GetValue(@"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\BIOS", "SystemProductName", null) as string
        ?? string.Empty;

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
        // library still exposes (load) and never publish physical readings: the library reports
        // temperature/power/voltage from virtual MSRs (0 °C, 0 W, constant volts) with status ok.
        catalog.ShouldNotContain(descriptor => descriptor.Metric == "temperature" || descriptor.Metric == "power" || descriptor.Metric == "voltage", summary);
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

        // Unhandled exceptions in the sidecar are reported on stderr with these codes before exiting;
        // controlled warnings (SENSOR_UPDATE_FAILED) are allowed on hosts without sensors.
        var errors = await stderr.WaitAsync(TimeSpan.FromSeconds(5), cancellationToken);
        errors.ShouldNotContain("AGENT_UNHANDLED_EXCEPTION", customMessage: errors);
        errors.ShouldNotContain("AGENT_UNOBSERVED_TASK", customMessage: errors);
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
