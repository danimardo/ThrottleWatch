using System;
using System.Collections.Generic;
using System.Linq;
using LibreHardwareMonitor.Hardware;
using Microsoft.Extensions.Logging;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

/// <summary>
/// Regression for the virtualized CI runner: LibreHardwareMonitorLib threw
/// NullReferenceException from Amd17Cpu.Update() and the sidecar died. Fails
/// without HardwareCollector.TryUpdate.
/// </summary>
public sealed class HardwareCollectorUpdateFailureTests
{
    [Fact]
    [Trait("Category", "Unit")]
    public void ReportsMissingReadingsAndLogsOnceWhenTheLibraryThrowsOnUpdate()
    {
        var hardware = new FakeHardware("/amdcpu/0", throwOnUpdate: true);
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(hardware), logger);
        collector.Open();

        var first = collector.ReadSample();
        var second = collector.ReadSample();

        first.Select(reading => reading.SensorId).ShouldBe(["/amdcpu/0/CPU Package", "/amdcpu/0/CPU Total"]);
        first.ShouldAllBe(reading => reading.Status == "missing" && reading.Number == null);
        second.ShouldAllBe(reading => reading.Status == "missing");
        logger.Entries.Count(entry => entry.Contains("SENSOR_UPDATE_FAILED")).ShouldBe(1);
        logger.Entries[0].ShouldContain("/amdcpu/0");
        logger.Entries[0].ShouldContain("InvalidOperationException");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void LogsAgainWhenTheFailureRelapsesAfterARecovery()
    {
        var hardware = new FakeHardware("/amdcpu/0", throwOnUpdate: true);
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(hardware), logger);
        collector.Open();

        _ = collector.ReadSample();
        hardware.ThrowOnUpdate = false;
        var recovered = collector.ReadSample();
        hardware.ThrowOnUpdate = true;
        _ = collector.ReadSample();

        recovered.First(reading => reading.SensorId == "/amdcpu/0/CPU Package").Status.ShouldBe("ok");
        logger.Entries.Count(entry => entry.Contains("SENSOR_UPDATE_FAILED")).ShouldBe(2);
    }

    private sealed class FakeSource(IHardware hardware) : IHardwareSource
    {
        public void Open() { }
        public void Close() { }
        public IEnumerable<IHardware> CpuHardware() => [hardware];
        public void Dispose() { }
    }

    private sealed class CapturingLogger : ILogger
    {
        public List<string> Entries { get; } = [];
        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;
        public bool IsEnabled(LogLevel logLevel) => true;
        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
            => Entries.Add($"{logLevel} {formatter(state, exception)}");
    }

    private sealed class FakeHardware(string identifier, bool throwOnUpdate) : IHardware
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

    private sealed class FakeSensor(string name, SensorType type, float value) : ISensor
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
}
