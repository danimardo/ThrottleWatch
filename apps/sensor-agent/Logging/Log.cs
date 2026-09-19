using System.Collections.Concurrent;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace ThrottleWatch.SensorAgent.Logging;

public static partial class Log
{
    private static readonly ILoggerFactory Factory = LoggerFactory.Create(builder =>
    {
        builder.ClearProviders();
        builder.AddProvider(new JsonStderrLoggerProvider());
        builder.SetMinimumLevel(LogLevel.Debug);
    });

    public static ILogger Create(string target) => Factory.CreateLogger(target);

    [LoggerMessage(EventId = 1001, Level = LogLevel.Error, Message = "{Code}: {Message}")]
    public static partial void UnhandledException(this ILogger logger, string Code, string Message, Exception exception);

    public static void AttachUnhandledExceptionHandlers(ILogger logger)
    {
        AppDomain.CurrentDomain.UnhandledException += (_, args) =>
        {
            if (args.ExceptionObject is Exception exception)
            {
                logger.UnhandledException("AGENT_UNHANDLED_EXCEPTION", "unhandled exception", exception);
            }
        };
        TaskScheduler.UnobservedTaskException += (_, args) =>
        {
            logger.UnhandledException("AGENT_UNOBSERVED_TASK", "unobserved task exception", args.Exception);
            args.SetObserved();
        };
    }
}

internal sealed class JsonStderrLoggerProvider : ILoggerProvider
{
    private readonly ConcurrentDictionary<string, JsonStderrLogger> loggers = new(StringComparer.Ordinal);

    public ILogger CreateLogger(string categoryName) => loggers.GetOrAdd(categoryName, name => new JsonStderrLogger(name));

    public void Dispose()
    {
        foreach (var logger in loggers.Values)
        {
            logger.Dispose();
        }
        loggers.Clear();
    }
}

internal sealed class JsonStderrLogger(string target) : ILogger, IDisposable
{
    private static readonly object Gate = new();
    private readonly StandardErrorWriter writer = new();

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull => NullScope.Instance;

    public bool IsEnabled(LogLevel logLevel) => logLevel >= LogLevel.Debug;

    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel))
        {
            return;
        }

        var rendered = formatter(state, exception);
        var separator = rendered.IndexOf(": ", StringComparison.Ordinal);
        var code = separator > 0 ? rendered[..separator] : "AGENT_LOG";
        var message = separator > 0 ? rendered[(separator + 2)..] : rendered;
        var payload = new
        {
            ts = DateTimeOffset.UtcNow,
            level = logLevel.ToString().ToLowerInvariant(),
            component = "agent",
            target,
            code,
            msg = message,
            session_id = (string?)null,
            protocol_version = 1,
            fields = new Dictionary<string, object?> { ["event_id"] = eventId.Id },
            err = exception is null ? null : new { code = "UNHANDLED_EXCEPTION", message_key = exception.GetType().Name, path = (string?)null, context = (object?)null }
        };
        var json = JsonSerializer.Serialize(payload);
        lock (Gate)
        {
            writer.WriteLine(json);
        }
    }

    private sealed class NullScope : IDisposable
    {
        public static readonly NullScope Instance = new();
        public void Dispose() { }
    }

    public void Dispose() => writer.Dispose();
}

internal sealed class StandardErrorWriter : IDisposable
{
#pragma warning disable RS0030
    private readonly StreamWriter writer = new(Console.OpenStandardError(), new System.Text.UTF8Encoding(false)) { AutoFlush = true };
#pragma warning restore RS0030

    public void WriteLine(string value) => writer.WriteLine(value);

    public void Dispose() => writer.Dispose();
}
