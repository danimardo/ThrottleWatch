namespace ThrottleWatch.SensorAgent.Protocol;

using System.Text.Json;
using System.Text.Json.Serialization;

public sealed record IpcEnvelope<TPayload>(
    [property: JsonPropertyName("protocol_version")] int ProtocolVersion,
    [property: JsonPropertyName("session_nonce")] string SessionNonce,
    [property: JsonPropertyName("sequence")] long Sequence,
    [property: JsonPropertyName("timestamp_utc")] string TimestampUtc,
    [property: JsonPropertyName("type")] string Type,
    [property: JsonPropertyName("payload")] TPayload Payload);

public sealed record HelloPayload(
    [property: JsonPropertyName("app_version")] string AppVersion,
    [property: JsonPropertyName("supported_protocols")] IReadOnlyList<int> SupportedProtocols);

public sealed record LowLevelAccess(
    [property: JsonPropertyName("state")] string State,
    [property: JsonPropertyName("provider")] string? Provider,
    [property: JsonPropertyName("details_code")] string? DetailsCode);

public sealed record HelloAckPayload(
    [property: JsonPropertyName("agent_version")] string AgentVersion,
    [property: JsonPropertyName("selected_protocol")] int SelectedProtocol,
    [property: JsonPropertyName("runtime")] string Runtime,
    [property: JsonPropertyName("low_level_access")] LowLevelAccess LowLevelAccess);

public static class ProtocolValidator
{
    public const int ProtocolVersion = 1;
    public const int MaxMessageBytes = 1024 * 1024;

    public static bool TryValidate(
        ReadOnlySpan<byte> raw,
        string expectedNonce,
        long? previousSequence,
        out long sequence)
    {
        sequence = 0;
        if (raw.Length > MaxMessageBytes || string.IsNullOrWhiteSpace(expectedNonce))
        {
            return false;
        }

        try
        {
            using var document = JsonDocument.Parse(raw.ToArray());
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object
                || !root.TryGetProperty("protocol_version", out var version)
                || version.GetInt32() != ProtocolVersion
                || !root.TryGetProperty("session_nonce", out var nonce)
                || nonce.GetString() != expectedNonce
                || !root.TryGetProperty("sequence", out var sequenceValue)
                || !sequenceValue.TryGetInt64(out sequence)
                || (previousSequence is not null && sequence <= previousSequence)
                || !root.TryGetProperty("type", out var type)
                || string.IsNullOrWhiteSpace(type.GetString())
                || !root.TryGetProperty("payload", out var payload)
                || payload.ValueKind != JsonValueKind.Object)
            {
                sequence = 0;
                return false;
            }

            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }
}
