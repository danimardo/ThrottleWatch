using System.Linq;
using System.Collections.Generic;
using Shouldly;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Tests.Normalization;

public sealed class SensorNormalizerTests
{
    [Fact]
    [Trait("Category", "Unit")]
    public void PrefersPackageOrTdieOverCoreAndTctl()
    {
        var values = new[]
        {
            new RawTemperature("core-0", "Core 0", 91, "core"),
            new RawTemperature("tdie", "Tdie", 86, "tdie"),
            new RawTemperature("tctl", "Tctl", 96, "tctl")
        };

        SensorNormalizer.SelectRepresentative(values)?.SourceId.ShouldBe("tdie");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void UsesMaximumCoreWhenPackageIsMissing()
    {
        var values = new[]
        {
            new RawTemperature("core-0", "Core 0", 70, "core"),
            new RawTemperature("core-1", "Core 1", 84, "core"),
            new RawTemperature("tctl", "Tctl", 95, "tctl")
        };

        SensorNormalizer.SelectRepresentative(values)?.SourceId.ShouldBe("core-1");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PrefersTctlWithOffsetAndMarksUnoffsetTctlAsSubstitute()
    {
        var values = new[]
        {
            new RawTemperature("tctl-offset", "Tctl", 90, "tctl", 5),
            new RawTemperature("tctl-raw", "Tctl", 95, "tctl")
        };

        var normalized = SensorNormalizer.NormalizeTemperatures(values);
        normalized.Single(value => value.SourceId == "tctl-offset").IsRepresentative.ShouldBeTrue();
        normalized.Single(value => value.SourceId == "tctl-raw").Quality.ShouldBe("substitute");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void ClassifiesHybridAndAmdTopologiesWithUnknownFallback()
    {
        var intelP = new[] { new LogicalProcessorInfo(0, 0, null), new LogicalProcessorInfo(1, 0, null) };
        var intelE = new[] { new LogicalProcessorInfo(2, 1, null), new LogicalProcessorInfo(3, 1, null) };
        var amd = new[] { new LogicalProcessorInfo(0, null, null) };
        var unknown = new[] { new LogicalProcessorInfo(0, null, null) };

        CpuTopologyClassifier.Classify("intel", intelP).ShouldBe(CpuGroupKind.P);
        CpuTopologyClassifier.Classify("intel", intelE).ShouldBe(CpuGroupKind.E);
        CpuTopologyClassifier.Classify("amd", amd).ShouldBe(CpuGroupKind.Homogeneous);
        CpuTopologyClassifier.Classify("intel", unknown).ShouldBe(CpuGroupKind.Unknown);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void NormalizesNonTemperatureMetricsWithoutCreatingValues()
    {
        var metadata = new Dictionary<string, object?> { ["provenance"] = "replay" };
        var normalized = SensorNormalizer.NormalizeMetrics(new[]
        {
            new RawMetricReading("load", "source", "Load", "load", "core", 82, null, "direct", metadata),
            new RawMetricReading("invalid", "source", "Temp", "temperature", "package", 180, null, "direct", metadata),
            new RawMetricReading("missing", "source", "Power", "power", "package", null, null, "direct", metadata)
        });

        normalized[0].Number.ShouldBe(82);
        normalized[1].Number.ShouldBeNull();
        normalized[2].Quality.ShouldBe("unknown");
        normalized[0].Metadata["provenance"].ShouldBe("replay");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void ParsesOnlyTheAllowListedLimitBitsAndReturnsAWriteBackMask()
    {
        var reasons = LimitReasonNormalizer.ReadIntelReasons(
            LimitReasonNormalizer.ThermalBit | LimitReasonNormalizer.PowerBit | (1u << 31));

        reasons.ThermalFlag.ShouldBeTrue();
        reasons.PowerFlag.ShouldBeTrue();
        reasons.ProchotFlag.ShouldBeFalse();
        reasons.ClearMask.ShouldBe(LimitReasonNormalizer.ThermalBit | LimitReasonNormalizer.PowerBit);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void AppliesTheAmdThermalEquivalenceOnlyForVersionedTables()
    {
        LimitReasonNormalizer.IsAmdThermalFlag("zen4", "thermal-limits-v1", 99, 90, 90, 90).ShouldBeTrue();
        LimitReasonNormalizer.IsAmdThermalFlag("zen4", "unknown", 99, 90, 90, 90).ShouldBeFalse();
        LimitReasonNormalizer.IsAmdThermalFlag("zen4", "thermal-limits-v1", 99, 96, 90, 90).ShouldBeFalse();
    }
}
