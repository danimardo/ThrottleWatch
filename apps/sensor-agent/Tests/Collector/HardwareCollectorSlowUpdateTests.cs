using System.Diagnostics;
using System.Linq;
using System.Threading;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

/// <summary>
/// T187 (2026-09-28): on a real AMD machine with advanced access, <c>Amd17Cpu.Update()</c> got stuck
/// for over a second as soon as a CPU load started, and the whole sample waited for it — the
/// application saw no sample at all, declared the sidecar stalled and killed it. A slow hardware
/// read must not hold the sample back: its sensors are reported stale until it finishes.
/// </summary>
public sealed class HardwareCollectorSlowUpdateTests
{
    [Fact]
    [Trait("Category", "Unit")]
    public void ReportsStaleReadingsWhileAnUpdateIsStuckAndUsesItOnceItFinishes()
    {
        using var gate = new ManualResetEventSlim(false);
        var hardware = new FakeHardware("/amdcpu/0") { BlockUpdate = gate };
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(hardware), logger, detectVendor: () => "amd");
        collector.Open();

        var watch = Stopwatch.StartNew();
        var whileStuck = collector.ReadSample();
        var stillStuck = collector.ReadSample();
        watch.ElapsedMilliseconds.ShouldBeLessThan(HardwareCollector.UpdateDeadlineMs * 3);

        whileStuck.ShouldAllBe(reading => reading.Status == "stale" && reading.Number == null);
        stillStuck.ShouldAllBe(reading => reading.Status == "stale");
        hardware.UpdateCount.ShouldBe(1, "a stuck update is waited for, not stacked with another one");
        logger.Entries.Count(entry => entry.Contains("SENSOR_UPDATE_PENDING")).ShouldBe(1);

        gate.Set();
        var recovered = whileStuck;
        SpinWait.SpinUntil(() =>
        {
            recovered = collector.ReadSample();
            return recovered.All(reading => reading.Status == "ok");
        }, 5000).ShouldBeTrue();

        recovered.First(reading => reading.SensorId == "/amdcpu/0/temperature/0").Number.ShouldBe(61f);
        logger.Entries.Count(entry => entry.Contains("SENSOR_UPDATE_FINISHED_LATE")).ShouldBe(1);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void AFastUpdateIsReadInTheSameSample()
    {
        var hardware = new FakeHardware("/amdcpu/0");
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(hardware), logger, detectVendor: () => "amd");
        collector.Open();

        var sample = collector.ReadSample();

        sample.ShouldAllBe(reading => reading.Status == "ok");
        logger.Entries.ShouldBeEmpty();
    }
}
