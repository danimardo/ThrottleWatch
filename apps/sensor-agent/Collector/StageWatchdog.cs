using System.Diagnostics;
using Microsoft.Extensions.Logging;
using ThrottleWatch.SensorAgent.Logging;

namespace ThrottleWatch.SensorAgent.Collector;

/// <summary>
/// Names the step the sidecar is stuck in (2026-09-28, T187): on one Intel machine with advanced
/// access, the elevated sidecar stops producing samples the moment a CPU load starts, and a new one
/// never finishes its handshake while the application's own load runs — with nothing on stderr,
/// so the application only ever saw silence. A dedicated thread (not the thread pool, which a
/// stalled process may not serve) reports any stage still running after
/// <see cref="ThresholdMs"/>, once, and again when it finally finishes.
/// </summary>
public static class StageWatchdog
{
    public const long ThresholdMs = 1000;
    private const int PollMs = 200;
    private static readonly object Gate = new();
    private static ILogger? logger;
    private static string? stage;
    private static long startedAt;
    private static bool reported;

    public static void Start(ILogger target)
    {
        lock (Gate)
        {
            if (logger is not null)
            {
                return;
            }
            logger = target;
        }
        var thread = new Thread(Watch) { IsBackground = true, Name = "stage-watchdog", Priority = ThreadPriority.AboveNormal };
        thread.Start();
    }

    /// <summary>Marks <paramref name="name"/> as running until the returned scope is disposed; nests.</summary>
    public static Scope Enter(string name)
    {
        lock (Gate)
        {
            var previous = (stage, startedAt, reported);
            stage = name;
            startedAt = Stopwatch.GetTimestamp();
            reported = false;
            return new Scope(previous);
        }
    }

    public readonly struct Scope : IDisposable
    {
        private readonly (string? Stage, long StartedAt, bool Reported) previous;

        internal Scope((string? Stage, long StartedAt, bool Reported) previous) => this.previous = previous;

        public void Dispose()
        {
            string? finished = null;
            long elapsed = 0;
            ILogger? target;
            lock (Gate)
            {
                if (reported && stage is not null)
                {
                    finished = stage;
                    elapsed = (long)Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds;
                }
                (stage, startedAt, reported) = previous;
                target = logger;
            }
            if (finished is not null)
            {
                target?.StageFinishedLate("AGENT_STAGE_FINISHED_LATE", finished, elapsed);
            }
        }
    }

    private static void Watch()
    {
        while (true)
        {
            Thread.Sleep(PollMs);
            string? stuck = null;
            long elapsed = 0;
            ILogger? target;
            lock (Gate)
            {
                target = logger;
                if (stage is not null && !reported)
                {
                    elapsed = (long)Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds;
                    if (elapsed >= ThresholdMs)
                    {
                        stuck = stage;
                        reported = true;
                    }
                }
            }
            if (stuck is not null)
            {
                target?.StageStuck("AGENT_STAGE_STUCK", stuck, elapsed);
            }
        }
    }
}
