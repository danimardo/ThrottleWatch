using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Protocol;

public sealed record CpuInfo(
    [property: JsonPropertyName("vendor")] string Vendor,
    [property: JsonPropertyName("display_name")] string DisplayName,
    [property: JsonPropertyName("logical_processors")] int LogicalProcessors,
    [property: JsonPropertyName("hybrid")] bool Hybrid,
    [property: JsonPropertyName("virtualized")] bool Virtualized);

public sealed record CpuGroup(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("kind")] string Kind,
    [property: JsonPropertyName("logical_count")] int LogicalCount);

public sealed record CapabilitiesPayload(
    [property: JsonPropertyName("cpu")] CpuInfo Cpu,
    [property: JsonPropertyName("groups")] IReadOnlyList<CpuGroup> Groups,
    [property: JsonPropertyName("sensors")] IReadOnlyList<SensorInfo> Sensors);

public sealed record StartedPayload(
    [property: JsonPropertyName("interval_ms")] int IntervalMs,
    [property: JsonPropertyName("detail")] string Detail);

public sealed record SamplePayload(
    [property: JsonPropertyName("monotonic_ms")] long MonotonicMs,
    [property: JsonPropertyName("duration_ms")] long DurationMs,
    [property: JsonPropertyName("values")] IReadOnlyList<SampleValue> Values);

public sealed record EmptyOutPayload;

/// <summary>
/// One protocol session of the sidecar (contracts/ipc-protocol.md): hello → hello_ack +
/// capabilities, then start / set_rate / snapshot / stop / shutdown, and periodic samples.
/// Only "representative" detail is offered; the effective parameters go back in `started`.
/// Every public member takes the same lock: the reader task and the sampling timer share it.
/// </summary>
public sealed class SidecarSession
{
    public const int MinIntervalMs = 250;
    public const int MaxIntervalMs = 10_000;
    public const int DefaultIntervalMs = 1_000;

    private readonly object gate = new();
    private readonly ISensorSource source;
    private readonly HandshakeProcessor handshake;
    private readonly Func<DateTimeOffset> utcNow;
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private string? nonce;
    private long sequence;
    private long? lastReceived;
    private NormalizedCatalog? catalog;
    private IReadOnlyList<SensorDescriptor> rawCatalog = [];
    private string detail = CatalogNormalizer.DetailRepresentative;
    private bool sampling;
    private int intervalMs = DefaultIntervalMs;

    public SidecarSession(ISensorSource source, Func<DateTimeOffset>? utcNow = null)
    {
        this.source = source;
        handshake = new HandshakeProcessor(source);
        this.utcNow = utcNow ?? (() => DateTimeOffset.UtcNow);
    }

    public bool Sampling
    {
        get
        {
            lock (gate)
            {
                return sampling;
            }
        }
    }

    public int IntervalMs
    {
        get
        {
            lock (gate)
            {
                return intervalMs;
            }
        }
    }

    public bool ShutdownRequested { get; private set; }

    /// <summary>Handles one inbound line and returns the lines to write to stdout, in order.</summary>
    public IReadOnlyList<string> Handle(string line)
    {
        lock (gate)
        {
            var raw = Encoding.UTF8.GetBytes(line);
            if (nonce is null)
            {
                return HandleHello(raw);
            }

            if (!ProtocolValidator.TryValidate(raw, nonce, lastReceived, out var received))
            {
                return [];
            }

            lastReceived = received;
            using var document = JsonDocument.Parse(raw);
            var type = document.RootElement.GetProperty("type").GetString();
            var payload = document.RootElement.GetProperty("payload");
            return type switch
            {
                "start" => Start(payload),
                "set_rate" => SetRate(payload),
                "snapshot" => catalog is null ? [] : [Sample()],
                "stop" => Stop(),
                "shutdown" => Shutdown(),
                _ => []
            };
        }
    }

    /// <summary>One periodic sample, or null when sampling is not running.</summary>
    public string? NextSample()
    {
        lock (gate)
        {
            return sampling && catalog is not null ? Sample() : null;
        }
    }

    private List<string> HandleHello(byte[] raw)
    {
        if (!handshake.TryProcess(raw, out var ack))
        {
            return [];
        }

        using var document = JsonDocument.Parse(raw);
        nonce = document.RootElement.GetProperty("session_nonce").GetString();
        lastReceived = document.RootElement.GetProperty("sequence").GetInt64();
        sequence = 1; // hello_ack used sequence 0
        rawCatalog = source.ReadCatalog();
        catalog = CatalogNormalizer.Build(rawCatalog, detail);
        return [Encoding.UTF8.GetString(ack), Envelope("capabilities", BuildCapabilities())];
    }

    private CapabilitiesPayload BuildCapabilities()
    {
        var vendor = LowLevelAccessProbe.DetectCpuVendor();
        var groups = ReadGroups(vendor);
        return new CapabilitiesPayload(
            new CpuInfo(
                vendor is "intel" or "amd" ? vendor : "unknown",
                source.CpuDisplayName,
                Environment.ProcessorCount,
                groups.Count(group => group.Kind != "homogeneous" && group.Kind != "unknown") > 1,
                HostVirtualization.IsVirtualized()),
            groups,
            catalog!.Sensors);
    }

    private static List<CpuGroup> ReadGroups(string vendor)
    {
        var groups = new List<CpuGroup>();
        foreach (var (kind, processors) in CpuTopologyClassifier.ReadGroups(vendor))
        {
            var (id, name) = kind switch
            {
                CpuGroupKind.P => ("p", "p"),
                CpuGroupKind.E => ("e", "e"),
                CpuGroupKind.LpE => ("lp-e", "lp_e"),
                CpuGroupKind.Homogeneous => ("all", "homogeneous"),
                _ => ("unknown", "unknown")
            };
            groups.Add(new CpuGroup(id, name, processors.Count));
        }

        if (groups.Count == 0)
        {
            groups.Add(new CpuGroup("all", "unknown", Environment.ProcessorCount));
        }

        return groups;
    }

    private List<string> Start(JsonElement payload)
    {
        intervalMs = ClampInterval(payload);
        sampling = true;
        var requested = payload.TryGetProperty("detail", out var value) && value.GetString() == CatalogNormalizer.DetailPerCore
            ? CatalogNormalizer.DetailPerCore
            : CatalogNormalizer.DetailRepresentative; // per_group is not offered: the effective detail is reported
        var lines = new List<string>();
        var changed = requested != detail;
        detail = requested;
        lines.Add(Envelope("started", new StartedPayload(intervalMs, detail)));
        if (changed)
        {
            // The contract re-emits capabilities after a start with a different detail.
            catalog = CatalogNormalizer.Build(rawCatalog, detail);
            lines.Add(Envelope("capabilities", BuildCapabilities()));
        }

        return lines;
    }

    private List<string> SetRate(JsonElement payload)
    {
        intervalMs = ClampInterval(payload);
        return [Envelope("started", new StartedPayload(intervalMs, detail))];
    }

    private List<string> Stop()
    {
        sampling = false;
        return [Envelope("stopped", new EmptyOutPayload())];
    }

    private List<string> Shutdown()
    {
        sampling = false;
        ShutdownRequested = true;
        return [Envelope("stopped", new EmptyOutPayload())];
    }

    private int ClampInterval(JsonElement payload) =>
        payload.TryGetProperty("interval_ms", out var value) && value.TryGetInt32(out var requested)
            ? Math.Clamp(requested, MinIntervalMs, MaxIntervalMs)
            : intervalMs;

    private string Sample()
    {
        var started = clock.ElapsedMilliseconds;
        var readings = source.ReadSample();
        var values = catalog!.Map(readings);
        var duration = Math.Min(clock.ElapsedMilliseconds - started, 10_000);
        return Envelope("sample", new SamplePayload(started, duration, values));
    }

    private string Envelope<TPayload>(string type, TPayload payload) =>
        JsonSerializer.Serialize(new IpcEnvelope<TPayload>(
            ProtocolValidator.ProtocolVersion,
            nonce!,
            sequence++,
            utcNow().UtcDateTime.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", System.Globalization.CultureInfo.InvariantCulture),
            type,
            payload));
}
