using System;
using System.Collections.Generic;
using LibreHardwareMonitor.Hardware;
using Microsoft.Extensions.Logging;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

internal sealed class FakeSource(IHardware hardware) : IHardwareSource
{
    public void Open() { }
    public void Close() { }
    public IEnumerable<IHardware> CpuHardware() => [hardware];
    public void Dispose() { }
}

internal sealed class CapturingLogger : ILogger
{
    public List<string> Entries { get; } = [];
    public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;
    public bool IsEnabled(LogLevel logLevel) => true;
    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
        => Entries.Add($"{logLevel} {formatter(state, exception)}");
}

internal sealed class FakeHardware(string identifier, bool throwOnUpdate = false) : IHardware
{
    public bool ThrowOnUpdate { get; set; } = throwOnUpdate;
    public HardwareType HardwareType => HardwareType.Cpu;
    public Identifier Identifier { get; } = new Identifier(identifier.TrimStart('/').Split('/'));
    public string Name { get; set; } = "Fake CPU";
    public IHardware? Parent => null;
    public IDictionary<string, string> Properties { get; } = new Dictionary<string, string>();
    public ISensor[] Sensors { get; } =
    [
        new FakeSensor("CPU Package", SensorType.Temperature, 61f),
        new FakeSensor("CPU Total", SensorType.Load, 12f)
    ];
    public IHardware[] SubHardware => [];
    public event SensorEventHandler? SensorAdded { add { } remove { } }
    public event SensorEventHandler? SensorRemoved { add { } remove { } }
    public void Accept(IVisitor visitor) { }
    public string GetReport() => string.Empty;
    public void Traverse(IVisitor visitor) { }
    public void Update()
    {
        if (ThrowOnUpdate)
        {
            throw new InvalidOperationException("simulated library failure");
        }
    }
}

internal sealed class FakeSensor(string name, SensorType type, float value) : ISensor
{
    public IControl? Control => null;
    public IHardware Hardware => null!;
    public Identifier Identifier { get; } = new Identifier(name);
    public int Index => 0;
    public bool IsDefaultHidden => false;
    public float? Max => value;
    public float? Min => value;
    public string Name { get; set; } = name;
    public IReadOnlyList<IParameter> Parameters => [];
    public SensorType SensorType => type;
    public float? Value => value;
    public IEnumerable<SensorValue> Values => [];
    public TimeSpan ValuesTimeWindow { get; set; }
    public void Accept(IVisitor visitor) { }
    public void ClearValues() { }
    public void ResetMax() { }
    public void ResetMin() { }
    public void Traverse(IVisitor visitor) { }
}
