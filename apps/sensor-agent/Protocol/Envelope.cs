namespace ThrottleWatch.SensorAgent.Protocol;

using System.Linq;
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
                || payload.ValueKind != JsonValueKind.Object
                || !IsKnownType(type.GetString()!)
                || !IsValidPayload(type.GetString()!, payload))
            {
                sequence = 0;
                return false;
            }

            return true;
        }
        catch (Exception exception)
            when (exception is JsonException or InvalidOperationException or FormatException)
        {
            return false;
        }
    }

    private static bool IsKnownType(string type) => type is
        "hello" or "hello_ack" or "capabilities" or "start" or "started" or "set_rate" or
        "snapshot" or "sample" or "stop" or "stopped" or "shutdown" or "error";

    private static bool IsValidPayload(string type, JsonElement payload) => type switch
    {
        "hello" => payload.TryGetProperty("app_version", out var appVersion)
            && appVersion.ValueKind == JsonValueKind.String
            && !string.IsNullOrWhiteSpace(appVersion.GetString())
            && payload.TryGetProperty("supported_protocols", out var protocols)
            && protocols.ValueKind == JsonValueKind.Array
            && protocols.EnumerateArray().Any(protocol => protocol.TryGetInt32(out var value) && value == 1),
        "capabilities" => payload.TryGetProperty("cpu", out var cpu)
            && cpu.ValueKind == JsonValueKind.Object
            && payload.TryGetProperty("groups", out var groups)
            && groups.ValueKind == JsonValueKind.Array
            && payload.TryGetProperty("sensors", out var sensors)
            && sensors.ValueKind == JsonValueKind.Array,
        "sample" => IsValidSamplePayload(payload),
        "error" => IsValidErrorPayload(payload),
        _ => true
    };

    private static bool IsValidSamplePayload(JsonElement payload)
    {
        if (!payload.TryGetProperty("monotonic_ms", out var monotonic)
            || !monotonic.TryGetInt64(out _)
            || !payload.TryGetProperty("duration_ms", out var duration)
            || !duration.TryGetInt64(out var durationValue)
            || durationValue < 0
            || durationValue > 10_000
            || !payload.TryGetProperty("values", out var values)
            || values.ValueKind != JsonValueKind.Array)
        {
            return false;
        }

        foreach (var value in values.EnumerateArray())
        {
            if (value.ValueKind != JsonValueKind.Object
                || !value.TryGetProperty("sensor_id", out var sensorId)
                || string.IsNullOrWhiteSpace(sensorId.GetString())
                || !value.TryGetProperty("status", out var status))
            {
                return false;
            }

            var statusValue = status.GetString();
            var hasNumber = value.TryGetProperty("number", out var number)
                && number.ValueKind == JsonValueKind.Number;
            var hasBoolean = value.TryGetProperty("boolean", out var boolean)
                && (boolean.ValueKind is JsonValueKind.True or JsonValueKind.False);
            if (statusValue == "ok" ? hasNumber == hasBoolean : hasNumber || hasBoolean)
            {
                return false;
            }

            if (statusValue is not ("ok" or "missing" or "stale" or "invalid" or "unsupported"))
            {
                return false;
            }
        }

        return true;
    }

    private static bool IsValidErrorPayload(JsonElement payload)
    {
        if (!payload.TryGetProperty("code", out var code)
            || code.ValueKind != JsonValueKind.String
            || string.IsNullOrWhiteSpace(code.GetString())
            || code.GetString()!.Any(character =>
                !(character is >= 'A' and <= 'Z')
                && !(character is >= '0' and <= '9')
                && character != '_')
            || !payload.TryGetProperty("severity", out var severity)
            || severity.GetString() is not ("info" or "recoverable" or "fatal")
            || !payload.TryGetProperty("message_key", out var messageKey)
            || messageKey.ValueKind != JsonValueKind.String
            || string.IsNullOrWhiteSpace(messageKey.GetString()))
        {
            return false;
        }

        return true;
    }
}
