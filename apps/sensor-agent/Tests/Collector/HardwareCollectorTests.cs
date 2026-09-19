using LibreHardwareMonitor.Hardware;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

public sealed class HardwareCollectorTests
{
    [Theory]
    [InlineData(SensorType.Temperature, 80, true)]
    [InlineData(SensorType.Temperature, 200, false)]
    [InlineData(SensorType.Load, 101, false)]
    [InlineData(SensorType.Power, 65, true)]
    [InlineData(SensorType.Voltage, -1, false)]
    [Trait("Category", "Unit")]
    public void RejectsImpossibleSensorValues(SensorType type, float value, bool expected)
    {
        HardwareCollector.IsPhysicallyValid(type, value).ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Integration")]
    public void KeepsComputerOpenAcrossCatalogProbeAndCoverageRecheck()
    {
        using var collector = new HardwareCollector();
        collector.Open();

        _ = collector.ReadCatalog();
        var afterCatalog = collector.ProbeLowLevelAfterCatalog();
        var afterRecheck = collector.RecheckCoverage();

        collector.IsOpen.ShouldBeTrue();
        afterCatalog.State.ShouldNotBeNullOrWhiteSpace();
        afterRecheck.State.ShouldNotBeNullOrWhiteSpace();
    }
}
