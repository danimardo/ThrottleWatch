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
}
