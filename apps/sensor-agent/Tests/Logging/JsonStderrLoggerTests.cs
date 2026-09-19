using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Shouldly;
using ThrottleWatch.SensorAgent.Logging;

namespace ThrottleWatch.SensorAgent.Tests.Logging;

/// <summary>
/// Every line the sidecar writes to stderr must satisfy the log-event contract
/// (packages/contracts/schemas/log-event.schema.json): the level enum, the
/// required keys and the shape of `err`. Checked here without a schema library
/// by reading the contract's own enum and required list.
/// </summary>
public sealed class JsonStderrLoggerTests
{
    private static readonly JsonDocument Schema = JsonDocument.Parse(
        File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Schemas", "log-event.schema.json")));

    [Theory]
    [InlineData(LogLevel.Debug, "debug")]
    [InlineData(LogLevel.Information, "info")]
    [InlineData(LogLevel.Warning, "warn")]
    [InlineData(LogLevel.Error, "error")]
    [InlineData(LogLevel.Critical, "error")]
    [Trait("Category", "Unit")]
    public void WritesLevelsFromTheContractEnum(LogLevel level, string expected)
    {
        var allowed = Schema.RootElement.GetProperty("properties").GetProperty("level").GetProperty("enum")
            .EnumerateArray().Select(value => value.GetString()).ToArray();
        allowed.ShouldContain(expected);

        var line = Render(level, "SENSOR_UPDATE_FAILED: hardware /amdcpu/0 update failed", null);
        line.RootElement.GetProperty("level").GetString().ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void WritesEveryRequiredKeyAndTheEventCodeInsideErr()
    {
        var required = Schema.RootElement.GetProperty("required").EnumerateArray().Select(value => value.GetString()!).ToArray();
        var line = Render(LogLevel.Warning, "SENSOR_UPDATE_FAILED: hardware /amdcpu/0 update failed (InvalidOperationException)", new InvalidOperationException("boom"));

        foreach (var key in required)
        {
            line.RootElement.TryGetProperty(key, out _).ShouldBeTrue($"missing required key {key}");
        }
        line.RootElement.GetProperty("code").GetString().ShouldBe("SENSOR_UPDATE_FAILED");
        line.RootElement.GetProperty("component").GetString().ShouldBe("agent");
        var err = line.RootElement.GetProperty("err");
        err.GetProperty("code").GetString().ShouldBe("SENSOR_UPDATE_FAILED");
        err.GetProperty("message_key").GetString().ShouldBe("InvalidOperationException");
        err.EnumerateObject().Select(property => property.Name).ShouldBe(["code", "message_key", "path", "context"]);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void DropsTraceBelowTheMinimumLevel()
    {
        var buffer = new StringWriter();
        using (var logger = new JsonStderrLogger("test", buffer))
        {
            logger.Log(LogLevel.Trace, new EventId(1), "AGENT_TRACE: noise", null, (state, _) => state);
        }

        buffer.ToString().ShouldBeEmpty();
    }

    private static JsonDocument Render(LogLevel level, string rendered, Exception? exception)
    {
        var buffer = new StringWriter();
        using (var logger = new JsonStderrLogger("test", buffer))
        {
            logger.Log(level, new EventId(1002), rendered, exception, (state, _) => state);
        }

        return JsonDocument.Parse(buffer.ToString().Trim());
    }
}
