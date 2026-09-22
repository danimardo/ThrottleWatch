using System.Linq;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

public sealed class HostVirtualizationTests
{
    [Theory]
    [InlineData(true, "Virtual Machine", true)]
    [InlineData(true, "VMware7,1", true)]
    [InlineData(true, "VirtualBox", true)]
    [InlineData(true, "Standard PC (Q35 + ICH9, 2009) QEMU", true)]
    [InlineData(true, "virtual machine", true)]
    // Bare metal with virtualization-based security sets the hypervisor bit but names real hardware.
    [InlineData(true, "MS-7B89", false)]
    [InlineData(false, "Virtual Machine", false)]
    [InlineData(true, "", false)]
    [Trait("Category", "Unit")]
    public void ClassifiesHypervisorBitAndProductName(bool hypervisorBit, string product, bool expected)
    {
        HostVirtualization.Classify(hypervisorBit, product).ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void HidesPhysicalSensorsOnAVirtualizedHostAndKeepsLoad()
    {
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(new FakeHardware("/amdcpu/0")), logger, () => true, () => "amd");
        collector.Open();

        var catalog = collector.ReadCatalog();
        var sample = collector.ReadSample();

        catalog.Select(descriptor => descriptor.Metric).ShouldBe(["load"]);
        sample.Select(reading => reading.SensorId).ShouldBe(["/amdcpu/0/load/1"]);
        logger.Entries.Count(entry => entry.Contains("HOST_VIRTUALIZED")).ShouldBe(1);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void MsrSensorsAreStillDeclaredForIntelOnAVirtualizedHost()
    {
        // The four limit-reason flags and the power-limit descriptor are declared unconditionally
        // for Intel (IntelLimitCatalog.Descriptors), unlike the LHM physical sensors this collector
        // hides under virtualization: a guest never fabricates a value for them either way, it just
        // reports "missing" per sample when PawnIO cannot really back them (T-INT-005, T028b).
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(new FakeHardware("/intelcpu/0")), logger, () => true, () => "intel");
        collector.Open();

        var metrics = collector.ReadCatalog().Select(descriptor => descriptor.Metric).ToArray();

        metrics.ShouldContain("load");
        metrics.ShouldNotContain("temperature"); // hidden physical LHM sensor
        foreach (var flag in new[] { "thermal_flag", "prochot_flag", "power_flag", "current_flag" })
        {
            metrics.ShouldContain(flag);
        }
        metrics.ShouldContain("power_limit_direct");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void KeepsEverySensorOnARealHost()
    {
        var logger = new CapturingLogger();
        using var collector = new HardwareCollector(new FakeSource(new FakeHardware("/amdcpu/0")), logger, () => false, () => "amd");
        collector.Open();

        collector.ReadCatalog().Select(descriptor => descriptor.Metric).ShouldBe(["temperature", "load"]);
        collector.ReadSample().Count.ShouldBe(2);
        logger.Entries.ShouldNotContain(entry => entry.Contains("HOST_VIRTUALIZED"));
    }
}
