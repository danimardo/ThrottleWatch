using Shouldly;
using ThrottleWatch.SensorAgent.Collector;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

public sealed class LowLevelAccessProbeTests
{
    [Theory]
    [InlineData(0x00640000UL, 100, 0)]
    [InlineData(0x035A0000UL, 90, 3)]
    // Bit 28 (reserved, outside the 4-bit 27:24 field) set: must not leak into the decoded offset.
    [InlineData(0x1F640000UL, 100, 15)]
    [Trait("Category", "Unit")]
    public void DecodesIntelTemperatureTarget(ulong value, int expectedTjMax, int expectedOffset)
    {
        LowLevelAccessProbe.DecodeTjMaxCelsius(value).ShouldBe(expectedTjMax);
        LowLevelAccessProbe.DecodeTccOffsetCelsius(value).ShouldBe(expectedOffset);
    }

    [Theory]
    [InlineData(0x00640000UL, true)]
    [InlineData(0x00100000UL, false)]
    [InlineData(0x00FF0000UL, false)]
    [Trait("Category", "Unit")]
    public void RejectsImplausibleIntelTemperatureTargets(ulong value, bool expected)
    {
        LowLevelAccessProbe.IsPlausibleTemperatureTarget(value).ShouldBe(expected);
    }

    [Theory]
    [InlineData("GenuineIntel", "intel")]
    [InlineData("AuthenticAMD", "amd")]
    [InlineData("HygonGenuine", "unknown")]
    [InlineData("", "unknown")]
    [Trait("Category", "Unit")]
    public void ClassifiesCpuIdVendorString(string vendorId, string expected)
    {
        LowLevelAccessProbe.ClassifyCpuVendor(vendorId).ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void DetectsVendorOnThisMachineWithoutOpeningLibreHardwareMonitor()
    {
        // CPUID is read directly; on any x86-64 box the vendor must be one of the known values.
        LowLevelAccessProbe.DetectCpuVendor().ShouldBeOneOf("intel", "amd", "unknown");
    }

    [Theory]
    [InlineData(0UL, 0UL, 1000UL, 1000UL, 1.0)]
    [InlineData(0UL, 0UL, 500UL, 1000UL, 0.5)]
    [InlineData(1_000_000UL, 2_000_000UL, 1_371_000UL, 3_000_000UL, 0.371)]
    [Trait("Category", "Unit")]
    public void ComputesAperfMperfRatioFromCounterDeltas(
        ulong aperf0, ulong mperf0, ulong aperf1, ulong mperf1, double expected)
    {
        var ratio = LowLevelAccessProbe.ComputeAperfMperfRatio(aperf0, mperf0, aperf1, mperf1);
        ratio.ShouldNotBeNull();
        ratio!.Value.ShouldBe(expected, 0.0005);
    }

    [Theory]
    // MPERF delta zero: no elapsed reference clock, ratio undefined.
    [InlineData(0UL, 500UL, 100UL, 500UL)]
    // MPERF went backwards: counter reset or read order broken, never a valid sample.
    [InlineData(0UL, 500UL, 100UL, 100UL)]
    [Trait("Category", "Unit")]
    public void RejectsAperfMperfRatioWithoutAPositiveMperfDelta(
        ulong aperf0, ulong mperf0, ulong aperf1, ulong mperf1)
    {
        LowLevelAccessProbe.ComputeAperfMperfRatio(aperf0, mperf0, aperf1, mperf1).ShouldBeNull();
    }
}
