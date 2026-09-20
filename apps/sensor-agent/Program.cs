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
        var session = new SidecarSession(collector);
        var logger = Log.Create("sensor-agent");
        Log.AttachUnhandledExceptionHandlers(logger);

        // Two writers share stdout: the reader task (replies) and the sampling timer.
        using var writeGate = new SemaphoreSlim(1, 1);
        async Task WriteAsync(IReadOnlyList<string> lines)
        {
            await writeGate.WaitAsync();
            try
            {
                foreach (var line in lines)
                {
                    await streams.WriteLineAsync(line);
                }
            }
            finally
            {
                writeGate.Release();
            }
        }

        using var stop = new CancellationTokenSource();
        var sampler = Task.Run(async () =>
        {
            while (!stop.IsCancellationRequested)
            {
                try
                {
                    await Task.Delay(session.IntervalMs, stop.Token);
                    if (session.NextSample() is { } sample)
                    {
                        await WriteAsync([sample]);
                    }
                }
                catch (OperationCanceledException)
                {
                    break;
                }
                catch (Exception exception) when (exception is not OutOfMemoryException)
                {
                    logger.SamplingFailed("SAMPLING_FAILED", exception.GetType().Name, exception);
                }
            }
        });

        while (await streams.ReadLineAsync(CancellationToken.None) is { } line)
        {
            await WriteAsync(session.Handle(line));
            if (session.ShutdownRequested)
            {
                break;
            }
        }

        await stop.CancelAsync();
        await sampler;
        return 0;
    }
}
