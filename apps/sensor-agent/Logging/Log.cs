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

    [LoggerMessage(EventId = 1002, Level = LogLevel.Warning, Message = "{Code}: hardware {HardwareId} update failed ({ExceptionType})")]
    public static partial void HardwareUpdateFailed(this ILogger logger, string Code, string HardwareId, string ExceptionType, Exception exception);

    [LoggerMessage(EventId = 1003, Level = LogLevel.Information, Message = "{Code}: physical sensors are hidden on a virtualized host")]
    public static partial void HostVirtualized(this ILogger logger, string Code);

    [LoggerMessage(EventId = 1004, Level = LogLevel.Warning, Message = "{Code}: hardware could not be opened ({ExceptionType})")]
    public static partial void HardwareOpenFailed(this ILogger logger, string Code, string ExceptionType, Exception exception);

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

internal sealed class JsonStderrLogger : ILogger, IDisposable
{
    private static readonly object Gate = new();
    private readonly string target;
    private readonly TextWriter writer;

    public JsonStderrLogger(string target)
        : this(target, new StandardErrorWriter())
    {
    }

    /// <summary>Seam for tests: writes the same JSON lines to any writer.</summary>
    internal JsonStderrLogger(string target, TextWriter writer)
    {
        this.target = target;
        this.writer = writer;
    }

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
            level = SchemaLevel(logLevel),
            component = "agent",
            target,
            code,
            msg = message,
            session_id = (string?)null,
            protocol_version = 1,
            fields = new Dictionary<string, object?> { ["event_id"] = eventId.Id },
            // err.code carries the event code: the exception's meaning depends on the event that reports it.
            err = exception is null ? null : new { code, message_key = exception.GetType().Name, path = (string?)null, context = (object?)null }
        };
        var json = JsonSerializer.Serialize(payload);
        lock (Gate)
        {
            writer.WriteLine(json);
        }
    }

    /// <summary>Maps the runtime levels to the five levels of the log-event contract (XVII).
    /// Critical has no counterpart and is reported as error.</summary>
    internal static string SchemaLevel(LogLevel level) => level switch
    {
        LogLevel.Trace => "trace",
        LogLevel.Debug => "debug",
        LogLevel.Information => "info",
        LogLevel.Warning => "warn",
        _ => "error"
    };

    private sealed class NullScope : IDisposable
    {
        public static readonly NullScope Instance = new();
        public void Dispose() { }
    }

    public void Dispose() => writer.Dispose();
}

internal sealed class StandardErrorWriter : TextWriter
{
#pragma warning disable RS0030
    private readonly StreamWriter writer = new(Console.OpenStandardError(), new System.Text.UTF8Encoding(false)) { AutoFlush = true };
#pragma warning restore RS0030

    public override System.Text.Encoding Encoding => writer.Encoding;

    public override void Write(char value) => writer.Write(value);

    public override void WriteLine(string? value) => writer.WriteLine(value);

    protected override void Dispose(bool disposing)
    {
        if (disposing)
        {
            writer.Dispose();
        }
        base.Dispose(disposing);
    }
}
