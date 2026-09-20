using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Protocol;

namespace ThrottleWatch.SensorAgent.Tests.Protocol;

public sealed class SidecarSessionTests
{
    private const string Nonce = "test-nonce";

    private sealed class FakeSource : ISensorSource
    {
        public int Samples { get; private set; }

        public string CpuDisplayName => "Fake CPU";

        public IReadOnlyList<SensorDescriptor> ReadCatalog() =>
        [
            new("/cpu/0/Core (Tctl/Tdie)", "/cpu/0", "Core (Tctl/Tdie)", "temperature", "celsius"),
            new("/cpu/0/CPU Total", "/cpu/0", "CPU Total", "load", "percent")
        ];

        public IReadOnlyList<SensorReading> ReadSample()
        {
            Samples++;
            return [new("/cpu/0/Core (Tctl/Tdie)", 60f + Samples, "ok"), new("/cpu/0/CPU Total", 10f, "ok")];
        }

        public LowLevelAccessProbeReport ProbeLowLevelAfterCatalog() =>
            new("amd", "missing", null, null, [], null, false, "PAWNIO_NOT_INSTALLED");

        public LowLevelAccessProbeReport RecheckCoverage() => ProbeLowLevelAfterCatalog();
    }

    private static string Inbound(string type, string payload, long sequence, string nonce = Nonce) =>
        "{\"protocol_version\":1,\"session_nonce\":\"" + nonce + "\",\"sequence\":" + sequence
        + ",\"timestamp_utc\":\"2026-09-20T10:00:00.000Z\",\"type\":\"" + type + "\",\"payload\":" + payload + "}";

    private static string Hello() =>
        Inbound("hello", "{\"app_version\":\"test\",\"supported_protocols\":[1]}", 0);

    private static JsonElement Parse(string line) => JsonDocument.Parse(line).RootElement;

    private static (SidecarSession Session, FakeSource Source) Opened()
    {
        var source = new FakeSource();
        var session = new SidecarSession(source, () => new DateTimeOffset(2026, 9, 20, 10, 0, 0, TimeSpan.Zero));
        session.Handle(Hello());
        return (session, source);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void HelloReturnsAckAndCapabilitiesWithTheNormalizedCatalog()
    {
        var session = new SidecarSession(new FakeSource());

        var lines = session.Handle(Hello());

        lines.Count.ShouldBe(2);
        Parse(lines[0]).GetProperty("type").GetString().ShouldBe("hello_ack");
        Parse(lines[0]).GetProperty("sequence").GetInt64().ShouldBe(0);
        var capabilities = Parse(lines[1]);
        capabilities.GetProperty("type").GetString().ShouldBe("capabilities");
        capabilities.GetProperty("sequence").GetInt64().ShouldBe(1);
        capabilities.GetProperty("session_nonce").GetString().ShouldBe(Nonce);
        var payload = capabilities.GetProperty("payload");
        payload.GetProperty("cpu").GetProperty("display_name").GetString().ShouldBe("Fake CPU");
        payload.GetProperty("cpu").GetProperty("logical_processors").GetInt32().ShouldBeGreaterThan(0);
        payload.GetProperty("groups").GetArrayLength().ShouldBeGreaterThan(0);
        payload.GetProperty("sensors").EnumerateArray().Select(sensor => sensor.GetProperty("id").GetString())
            .ShouldBe(["cpu.package.temp", "cpu.package.load"]);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void EverythingBeforeTheHelloIsIgnored()
    {
        var session = new SidecarSession(new FakeSource());

        session.Handle(Inbound("start", "{\"interval_ms\":1000}", 0)).ShouldBeEmpty();
        session.Handle("not json").ShouldBeEmpty();
        session.Sampling.ShouldBeFalse();
    }

    [Theory]
    [InlineData("{\"interval_ms\":100}", 250)]
    [InlineData("{\"interval_ms\":20000}", 10_000)]
    [InlineData("{\"interval_ms\":2000}", 2_000)]
    [InlineData("{}", 1_000)]
    [Trait("Category", "Unit")]
    public void StartConfirmsTheEffectiveIntervalWithinTheContractRange(string payload, int expected)
    {
        var (session, _) = Opened();

        var started = Parse(session.Handle(Inbound("start", payload, 1)).Single());

        started.GetProperty("type").GetString().ShouldBe("started");
        started.GetProperty("payload").GetProperty("interval_ms").GetInt32().ShouldBe(expected);
        started.GetProperty("payload").GetProperty("detail").GetString().ShouldBe("representative");
        session.IntervalMs.ShouldBe(expected);
        session.Sampling.ShouldBeTrue();
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void SamplesFlowOnlyWhileStartedAndCarryOkValuesWithIncreasingSequences()
    {
        var (session, _) = Opened();
        session.NextSample().ShouldBeNull();

        session.Handle(Inbound("start", "{\"interval_ms\":500}", 1));
        var first = Parse(session.NextSample()!);
        var second = Parse(session.NextSample()!);

        first.GetProperty("type").GetString().ShouldBe("sample");
        second.GetProperty("sequence").GetInt64().ShouldBe(first.GetProperty("sequence").GetInt64() + 1);
        var values = first.GetProperty("payload").GetProperty("values").EnumerateArray().ToArray();
        values.Select(value => value.GetProperty("sensor_id").GetString()).ShouldBe(["cpu.package.temp", "cpu.package.load"]);
        values.All(value => value.GetProperty("status").GetString() == "ok" && value.TryGetProperty("number", out _)).ShouldBeTrue();
        first.GetProperty("payload").GetProperty("duration_ms").GetInt64().ShouldBeInRange(0, 10_000);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void EveryOutboundMessageSatisfiesTheProtocolValidator()
    {
        var (session, _) = Opened();
        var lines = new List<string>();
        lines.AddRange(session.Handle(Inbound("start", "{\"interval_ms\":500}", 1)));
        lines.Add(session.NextSample()!);
        lines.AddRange(session.Handle(Inbound("snapshot", "{}", 2)));
        lines.AddRange(session.Handle(Inbound("stop", "{}", 3)));

        long? previous = 1; // hello_ack (0) and capabilities (1) were produced by Opened()
        foreach (var line in lines)
        {
            ProtocolValidator.TryValidate(Encoding.UTF8.GetBytes(line), Nonce, previous, out var sequence).ShouldBeTrue(line);
            previous = sequence;
        }
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void SetRateSnapshotStopAndShutdownFollowTheContract()
    {
        var (session, source) = Opened();
        session.Handle(Inbound("start", "{\"interval_ms\":1000}", 1));

        Parse(session.Handle(Inbound("set_rate", "{\"interval_ms\":300}", 2)).Single())
            .GetProperty("payload").GetProperty("interval_ms").GetInt32().ShouldBe(300);
        var before = source.Samples;
        Parse(session.Handle(Inbound("snapshot", "{}", 3)).Single()).GetProperty("type").GetString().ShouldBe("sample");
        source.Samples.ShouldBe(before + 1);

        Parse(session.Handle(Inbound("stop", "{}", 4)).Single()).GetProperty("type").GetString().ShouldBe("stopped");
        session.Sampling.ShouldBeFalse();
        session.NextSample().ShouldBeNull();

        session.ShutdownRequested.ShouldBeFalse();
        session.Handle(Inbound("shutdown", "{}", 5));
        session.ShutdownRequested.ShouldBeTrue();
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void InvalidMessagesAreDiscardedWithoutChangingState()
    {
        var (session, _) = Opened();
        session.Handle(Inbound("start", "{\"interval_ms\":1000}", 1));

        session.Handle(Inbound("stop", "{}", 2, nonce: "other-nonce")).ShouldBeEmpty(); // wrong nonce
        session.Handle(Inbound("stop", "{}", 1)).ShouldBeEmpty(); // sequence regression
        session.Handle("garbage").ShouldBeEmpty();
        session.Handle(Inbound("read_file", "{\"path\":\"C:/x\"}", 2)).ShouldBeEmpty(); // unknown command

        session.Sampling.ShouldBeTrue();
    }
}
