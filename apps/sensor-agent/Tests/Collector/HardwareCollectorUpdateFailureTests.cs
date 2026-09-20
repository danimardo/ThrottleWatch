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

        first.Select(reading => reading.SensorId).ShouldBe(["/amdcpu/0/temperature/0", "/amdcpu/0/load/1"]);
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

        recovered.First(reading => reading.SensorId == "/amdcpu/0/temperature/0").Status.ShouldBe("ok");
        logger.Entries.Count(entry => entry.Contains("SENSOR_UPDATE_FAILED")).ShouldBe(2);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void RunsWithNoHardwareWhenTheLibraryThrowsWhileOpening()
    {
        var source = new FakeSource(new FakeHardware("/amdcpu/0"), throwOnOpen: true);
        var logger = new CapturingLogger();
        var collector = new HardwareCollector(source, logger);

        collector.Open();

        collector.IsOpen.ShouldBeTrue();
        collector.ReadCatalog().ShouldBeEmpty();
        collector.ReadSample().ShouldBeEmpty();
        collector.ProbeLowLevelAfterCatalog().State.ShouldNotBeNullOrWhiteSpace();
        source.EnumerationCount.ShouldBe(0);
        logger.Entries.Count(entry => entry.Contains("SENSOR_OPEN_FAILED")).ShouldBe(1);
        logger.Entries[0].ShouldContain("InvalidOperationException");

        collector.Dispose();
        source.Closed.ShouldBeFalse();
    }
}
