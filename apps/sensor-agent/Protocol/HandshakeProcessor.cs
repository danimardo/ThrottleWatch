namespace ThrottleWatch.SensorAgent.Protocol;

using System.Text.Json;
using ThrottleWatch.SensorAgent.Collector;

public sealed class HandshakeProcessor
{
    private readonly ICollectorSession? collector;
    private string? sessionNonce;
    private long? lastReceivedSequence;
    private long responseSequence;

    public HandshakeProcessor(ICollectorSession? collector = null)
    {
        this.collector = collector;
    }

    public bool TryProcess(ReadOnlySpan<byte> raw, out byte[] response)
    {
        response = [];
        if (sessionNonce is null)
        {
            if (!TryReadHello(raw, out var hello, out var nonce))
            {
                return false;
            }

            sessionNonce = nonce;
            lastReceivedSequence = hello.Sequence;
            response = JsonSerializer.SerializeToUtf8Bytes(CreateAck(hello));
            return true;
        }

        if (!ProtocolValidator.TryValidate(raw, sessionNonce, lastReceivedSequence, out var sequence))
        {
            return false;
        }

        lastReceivedSequence = sequence;
        return false;
    }

    public LowLevelAccessProbeReport RecheckCoverage()
    {
        return collector?.RecheckCoverage() ?? LowLevelAccessProbe.Run();
    }

    private static bool TryReadHello(
        ReadOnlySpan<byte> raw,
        out IpcEnvelope<HelloPayload> hello,
        out string nonce)
    {
        hello = null!;
        nonce = string.Empty;
        if (raw.Length > ProtocolValidator.MaxMessageBytes)
        {
            return false;
        }

        try
        {
            using var document = JsonDocument.Parse(raw.ToArray());
            var root = document.RootElement;
            if (!root.TryGetProperty("session_nonce", out var nonceValue)
                || !root.TryGetProperty("type", out var typeValue)
                || typeValue.GetString() != "hello")
            {
                return false;
            }

            var parsed = JsonSerializer.Deserialize<IpcEnvelope<HelloPayload>>(raw);
            if (parsed is null
                || parsed.ProtocolVersion != ProtocolValidator.ProtocolVersion
                || string.IsNullOrWhiteSpace(nonceValue.GetString())
                || parsed.Payload.SupportedProtocols is null
                || !parsed.Payload.SupportedProtocols.Contains(ProtocolValidator.ProtocolVersion))
            {
                return false;
            }

            hello = parsed;
            nonce = nonceValue.GetString()!;
            return ProtocolValidator.TryValidate(raw, nonce, null, out _);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private IpcEnvelope<HelloAckPayload> CreateAck(IpcEnvelope<HelloPayload> hello)
    {
        var access = collector?.ProbeLowLevelAfterCatalog() ?? LowLevelAccessProbe.Run();
        return new IpcEnvelope<HelloAckPayload>(
            ProtocolValidator.ProtocolVersion,
            hello.SessionNonce,
            responseSequence++,
            DateTimeOffset.UtcNow.ToString("O"),
            "hello_ack",
            new HelloAckPayload(
                "0.1.0",
                ProtocolValidator.ProtocolVersion,
                ".NET",
                new LowLevelAccess(access.State, access.Provider, access.DetailsCode)));
    }
}
