using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using Shouldly;

namespace ThrottleWatch.SensorAgent.Tests.Integration;

public sealed class SensorAgentProcessTests
{
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
            document.RootElement.GetProperty("payload").GetProperty("low_level_access")
                .GetProperty("state").GetString().ShouldNotBeNullOrWhiteSpace();
        }

        await process.StandardInput.DisposeAsync();
        await process.WaitForExitAsync(cancellationToken)
            .WaitAsync(TimeSpan.FromSeconds(10), cancellationToken);
        process.ExitCode.ShouldBe(0);
    }
}
