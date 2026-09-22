using System.Collections.Generic;
using System.Linq;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Tests.Collector;

public sealed class IntelLimitCatalogTests
{
    private sealed class FakeMsrSession(Dictionary<uint, ulong> values) : IIntelMsrSession
    {
        public bool TryRead(uint msrAddress, out ulong value) => values.TryGetValue(msrAddress, out value);

        public bool TryWrite(uint msrAddress, ulong value) => false;

        public void Dispose()
        {
        }
    }

    private static ulong TemperatureTarget(int tjMaxCelsius, int tccOffsetCelsius = 0) =>
        ((ulong)tjMaxCelsius << 16) | ((ulong)tccOffsetCelsius << 24);

    [Fact]
    [Trait("Category", "Unit")]
    public void TemperatureMetadataIsNullWithoutAPlausibleTjMax()
    {
        // PawnIO answers "ok" with an all-zero register when access is really denied (T152):
        // a bare successful read is never enough, unlike a real hardware machine.
        var session = new FakeMsrSession(new Dictionary<uint, ulong> { [LimitReasonNormalizer.TemperatureTargetRegister] = 0 });

        var metadata = IntelLimitCatalog.ReadTemperatureMetadata(() => true, () => session);

        metadata.ShouldBeNull();
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void TemperatureMetadataIsNullWhenPawnIoIsNotInstalled()
    {
        var metadata = IntelLimitCatalog.ReadTemperatureMetadata(() => false, () => new FakeMsrSession([]));

        metadata.ShouldBeNull();
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void TemperatureMetadataCarriesTjMaxAndEffectiveLimitWhenPlausible()
    {
        var session = new FakeMsrSession(new Dictionary<uint, ulong>
        {
            [LimitReasonNormalizer.TemperatureTargetRegister] = TemperatureTarget(tjMaxCelsius: 100, tccOffsetCelsius: 5)
        });

        var metadata = IntelLimitCatalog.ReadTemperatureMetadata(() => true, () => session);

        metadata.ShouldNotBeNull();
        metadata!["tjmax_c"].ShouldBe(100.0);
        metadata["tcc_offset_c"].ShouldBe(5.0);
        metadata["thermal_limit_c"].ShouldBe(95.0);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void SampleIsMissingWithoutConfirmedAccessEvenIfTheRegisterWouldAnswer()
    {
        // The catalog never declared msr/temperature_target for this state (ReadTemperatureMetadata
        // returned null), so the sample must not invent it either — only the always-declared ids.
        var session = new FakeMsrSession(new Dictionary<uint, ulong>
        {
            [LimitReasonNormalizer.ReasonRegister] = LimitReasonNormalizer.ThermalBit,
            [LimitReasonNormalizer.PackagePowerLimitRegister] = 224 // 28 W
        });

        var readings = IntelLimitCatalog.ReadSample(accessConfirmed: false, () => true, () => session);

        readings.Select(reading => reading.SensorId).ShouldBe(
            [IntelLimitCatalog.ThermalFlagId, IntelLimitCatalog.ProchotFlagId, IntelLimitCatalog.PowerFlagId, IntelLimitCatalog.CurrentFlagId, IntelLimitCatalog.PowerLimitId],
            ignoreOrder: true);
        readings.ShouldAllBe(reading => reading.Status == "missing" && reading.Number == null && reading.Boolean == null);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void SampleIsMissingWhenPawnIoIsNotInstalledEvenIfAccessWasConfirmedEarlier()
    {
        var session = new FakeMsrSession(new Dictionary<uint, ulong> { [LimitReasonNormalizer.ReasonRegister] = LimitReasonNormalizer.ThermalBit });

        var readings = IntelLimitCatalog.ReadSample(accessConfirmed: true, () => false, () => session);

        readings.Select(reading => reading.SensorId).ShouldBe(
            [IntelLimitCatalog.TemperatureTargetId, IntelLimitCatalog.ThermalFlagId, IntelLimitCatalog.ProchotFlagId, IntelLimitCatalog.PowerFlagId, IntelLimitCatalog.CurrentFlagId, IntelLimitCatalog.PowerLimitId],
            ignoreOrder: true);
        readings.ShouldAllBe(reading => reading.Status == "missing");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void ConfirmedSampleReturnsTheFourFlagsAndTheEffectivePowerLimit()
    {
        var session = new FakeMsrSession(new Dictionary<uint, ulong>
        {
            [LimitReasonNormalizer.ReasonRegister] = LimitReasonNormalizer.ThermalBit | LimitReasonNormalizer.PowerBit,
            [LimitReasonNormalizer.PackagePowerLimitRegister] = 224 // PL1 = 224 / 8 = 28 W, PL2 absent
        });

        var readings = IntelLimitCatalog.ReadSample(accessConfirmed: true, () => true, () => session)
            .ToDictionary(reading => reading.SensorId);

        readings[IntelLimitCatalog.ThermalFlagId].Boolean.ShouldBe(true);
        readings[IntelLimitCatalog.ProchotFlagId].Boolean.ShouldBe(false);
        readings[IntelLimitCatalog.PowerFlagId].Boolean.ShouldBe(true);
        readings[IntelLimitCatalog.CurrentFlagId].Boolean.ShouldBe(false);
        readings[IntelLimitCatalog.PowerLimitId].Number.ShouldBe(28.0f);
        readings[IntelLimitCatalog.PowerLimitId].Status.ShouldBe("ok");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void ConfirmedSampleReportsPowerLimitAsMissingRatherThanZero()
    {
        var session = new FakeMsrSession(new Dictionary<uint, ulong>
        {
            [LimitReasonNormalizer.ReasonRegister] = 0,
            [LimitReasonNormalizer.PackagePowerLimitRegister] = 0
        });

        var readings = IntelLimitCatalog.ReadSample(accessConfirmed: true, () => true, () => session)
            .ToDictionary(reading => reading.SensorId);

        readings[IntelLimitCatalog.PowerLimitId].Status.ShouldBe("missing");
        readings[IntelLimitCatalog.PowerLimitId].Number.ShouldBeNull();
    }
}
