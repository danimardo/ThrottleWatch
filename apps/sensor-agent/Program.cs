using System.Text.Json;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Logging;
using ThrottleWatch.SensorAgent.Protocol;

namespace ThrottleWatch.SensorAgent;

internal static class DevConfiguration
{
    public static string? LogLevel()
    {
#if DEBUG || E2E
#pragma warning disable RS0030
        return Environment.GetEnvironmentVariable("TW_DEV_LOG_LEVEL");
#pragma warning restore RS0030
#else
        return null;
#endif
    }
}

internal static class Program
{
    private static async Task<int> Main(string[] args)
    {
        using var streams = new StandardStreams();
        if (args.Any(argument => string.Equals(argument, "--probe-low-level", StringComparison.Ordinal)))
        {
            using var probeCollector = new HardwareCollector();
            probeCollector.Open();
            await streams.WriteLineAsync(LowLevelAccessProbe.ToJson(probeCollector.ProbeLowLevelAfterCatalog()));
            return 0;
        }

        using var collector = new HardwareCollector();
        collector.Open();
        var processor = new HandshakeProcessor(collector);
        var logger = Log.Create("sensor-agent");
        Log.AttachUnhandledExceptionHandlers(logger);
        while (await streams.ReadLineAsync(CancellationToken.None) is { } line)
        {
            if (!processor.TryProcess(System.Text.Encoding.UTF8.GetBytes(line), out var response))
            {
                continue;
            }

            await streams.WriteLineAsync(System.Text.Encoding.UTF8.GetString(response));
        }

        return 0;
    }
}
