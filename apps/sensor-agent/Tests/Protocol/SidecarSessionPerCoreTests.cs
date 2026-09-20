using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Protocol;

namespace ThrottleWatch.SensorAgent.Tests.Protocol;

public sealed class SidecarSessionPerCoreTests
{
    private sealed class Source : ISensorSource
    {
        public string CpuDisplayName => "Fake CPU";

        public IReadOnlyList<SensorDescriptor> ReadCatalog() =>
        [
            new("/cpu/0/load/0", "/cpu/0", "CPU Total", "load", "percent"),
            new("/cpu/0/load/1", "/cpu/0", "CPU Core #1", "load", "percent")
        ];

        public IReadOnlyList<SensorReading> ReadSample() =>
            [new("/cpu/0/load/0", 20f, "ok"), new("/cpu/0/load/1", 40f, "ok")];

        public LowLevelAccessProbeReport ProbeLowLevelAfterCatalog() =>
            new("amd", "missing", null, null, [], null, false, "PAWNIO_NOT_INSTALLED");

        public LowLevelAccessProbeReport RecheckCoverage() => ProbeLowLevelAfterCatalog();
    }

    private static string Inbound(string type, string payload, long sequence) =>
        "{\"protocol_version\":1,\"session_nonce\":\"n\",\"sequence\":" + sequence
        + ",\"timestamp_utc\":\"2026-09-20T10:00:00.000Z\",\"type\":\"" + type + "\",\"payload\":" + payload + "}";

    private static JsonElement Parse(string line) => JsonDocument.Parse(line).RootElement;

    private static SidecarSession Opened()
    {
        var session = new SidecarSession(new Source());
        session.Handle(Inbound("hello", "{\"app_version\":\"t\",\"supported_protocols\":[1]}", 0));
        return session;
    }

    private static string[] SensorIds(JsonElement capabilities) =>
        capabilities.GetProperty("payload").GetProperty("sensors").EnumerateArray()
            .Select(sensor => sensor.GetProperty("id").GetString()!).ToArray();

    [Fact]
    [Trait("Category", "Unit")]
    public void StartWithPerCoreDetailReportsItAndReEmitsCapabilitiesWithCoreSensors()
    {
        var session = Opened();

        var lines = session.Handle(Inbound("start", "{\"interval_ms\":500,\"detail\":\"per_core\"}", 1));

        lines.Count.ShouldBe(2);
        Parse(lines[0]).GetProperty("type").GetString().ShouldBe("started");
        Parse(lines[0]).GetProperty("payload").GetProperty("detail").GetString().ShouldBe("per_core");
        Parse(lines[1]).GetProperty("type").GetString().ShouldBe("capabilities");
        SensorIds(Parse(lines[1])).ShouldBe(["cpu.package.load", "cpu.core.1.load"]);

        var sample = Parse(session.NextSample()!);
        sample.GetProperty("payload").GetProperty("values").EnumerateArray()
            .Select(value => value.GetProperty("sensor_id").GetString())
            .ShouldBe(["cpu.package.load", "cpu.core.1.load"]);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void RepeatingTheSameDetailDoesNotReEmitCapabilities()
    {
        var session = Opened();
        session.Handle(Inbound("start", "{\"detail\":\"per_core\"}", 1));

        session.Handle(Inbound("start", "{\"detail\":\"per_core\"}", 2)).Count.ShouldBe(1);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PerGroupIsNotOfferedAndFallsBackToRepresentativeInTheStartedReply()
    {
        var session = Opened();

        var lines = session.Handle(Inbound("start", "{\"detail\":\"per_group\"}", 1));

        lines.Count.ShouldBe(1); // representative was already effective
        Parse(lines.Single()).GetProperty("payload").GetProperty("detail").GetString().ShouldBe("representative");
    }
}
