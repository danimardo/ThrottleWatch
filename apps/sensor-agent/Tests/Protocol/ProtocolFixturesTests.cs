namespace ThrottleWatch.SensorAgent.Tests;

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Shouldly;
using ThrottleWatch.SensorAgent.Protocol;

public sealed class ProtocolFixturesTests
{
    public static IEnumerable<object[]> ContractFixtures()
    {
        var fixtureDirectory = Path.Combine(AppContext.BaseDirectory, "Fixtures");
        foreach (var path in Directory.EnumerateFiles(fixtureDirectory, "*.json").OrderBy(path => path))
        {
            yield return [Path.GetFileName(path)];
        }
    }

    [Theory]
    [Trait("Category", "Protocol")]
    [MemberData(nameof(ContractFixtures))]
    public void ContractFixtureMatchesAcceptanceConvention(string filename)
    {
        var path = Path.Combine(AppContext.BaseDirectory, "Fixtures", filename);
        var raw = File.ReadAllBytes(path);
        using var document = JsonDocument.Parse(raw);
        var root = document.RootElement;
        var nonce = root.GetProperty("session_nonce").GetString();
        var accepted = ProtocolValidator.TryValidate(raw, nonce!, null, out _);
        var expected = !filename.StartsWith("invalid-", StringComparison.Ordinal);

        accepted.ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Protocol")]
    public void ValidatorRejectsNonceAndSequenceViolations()
    {
        const string message = "{\"protocol_version\":1,\"session_nonce\":\"nonce\",\"sequence\":2,\"type\":\"sample\",\"payload\":{\"monotonic_ms\":1,\"duration_ms\":1,\"values\":[]}}";
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

        var replay = File.ReadAllBytes(path);
        processor.TryProcess(replay, out _).ShouldBeFalse();
    }
}
