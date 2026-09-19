namespace ThrottleWatch.SensorAgent.Tests;

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using Shouldly;
using ThrottleWatch.SensorAgent.Protocol;

public sealed class ProtocolFixturesTests
{
    public static IEnumerable<object[]> ValidFixtures()
    {
        yield return ["handshake.json"];
        yield return ["capabilities.json"];
        yield return ["sample.json"];
        yield return ["error.json"];
    }

    [Theory]
    [Trait("Category", "Protocol")]
    [MemberData(nameof(ValidFixtures))]
    public void CanonicalFixtureHasEnvelope(string filename)
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Fixtures", filename);
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var root = document.RootElement;

        root.GetProperty("protocol_version").GetInt32().ShouldBe(1);
        root.GetProperty("session_nonce").GetString().ShouldNotBeNullOrWhiteSpace();
        root.GetProperty("sequence").GetInt32().ShouldBeGreaterThanOrEqualTo(0);
        root.GetProperty("type").GetString().ShouldNotBeNullOrWhiteSpace();
        root.GetProperty("payload").ValueKind.ShouldBe(JsonValueKind.Object);
    }

    [Fact]
    [Trait("Category", "Protocol")]
    public void ValidatorRejectsNonceAndSequenceViolations()
    {
        const string message = "{\"protocol_version\":1,\"session_nonce\":\"nonce\",\"sequence\":2,\"type\":\"sample\",\"payload\":{}}";
        var bytes = System.Text.Encoding.UTF8.GetBytes(message);

        ProtocolValidator.TryValidate(bytes, "nonce", 1, out var sequence).ShouldBeTrue();
        sequence.ShouldBe(2);
        ProtocolValidator.TryValidate(bytes, "other", 1, out _).ShouldBeFalse();
        ProtocolValidator.TryValidate(bytes, "nonce", 2, out _).ShouldBeFalse();
    }

    [Fact]
    [Trait("Category", "Protocol")]
    public void HandshakeFixtureDeserializesToTypedDto()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Fixtures", "handshake.json");
        using var document = JsonDocument.Parse(File.ReadAllText(path));
        var envelope = JsonSerializer.Deserialize<IpcEnvelope<HelloPayload>>(document.RootElement.GetRawText());

        envelope.ShouldNotBeNull();
        envelope!.ProtocolVersion.ShouldBe(1);
        envelope.Type.ShouldBe("hello");
        envelope.Payload.SupportedProtocols.ShouldContain(1);
    }

    [Fact]
    [Trait("Category", "Protocol")]
    public void HandshakeProcessorCreatesAckAndRejectsReplay()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Fixtures", "handshake.json");
        var processor = new HandshakeProcessor();
        var raw = File.ReadAllBytes(path);

        processor.TryProcess(raw, out var response).ShouldBeTrue();
        var ack = JsonSerializer.Deserialize<IpcEnvelope<HelloAckPayload>>(response);
        ack.ShouldNotBeNull();
        ack!.Type.ShouldBe("hello_ack");
        ack.SessionNonce.ShouldBe("fixture-session-nonce-1");

        var replay = Encoding.UTF8.GetBytes(File.ReadAllText(path));
        processor.TryProcess(replay, out _).ShouldBeFalse();
    }
}
