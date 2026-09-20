using System.Collections.Generic;
using System.Linq;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Tests.Normalization;

public sealed class PerCoreCatalogTests
{
    private static SensorDescriptor Raw(string id, string name, string metric) =>
        new(id, "/cpu/0", name, metric, "unknown");

    private static SensorReading Ok(string id, float value) => new(id, value, "ok");

    private static readonly SensorDescriptor[] AmdCores =
    [
        Raw("/cpu/0/temperature/0", "Core (Tctl/Tdie)", "temperature"),
        Raw("/cpu/0/load/0", "CPU Total", "load"),
        Raw("/cpu/0/load/1", "CPU Core #1", "load"),
        Raw("/cpu/0/load/2", "CPU Core #2", "load"),
        Raw("/cpu/0/load/3", "CPU Core #1 Thread #1", "load"),
        Raw("/cpu/0/clock/1", "Core #1", "clock"),
        Raw("/cpu/0/clock/2", "Core #2", "clock"),
        Raw("/cpu/0/factor/1", "Core #1", "factor"),
        Raw("/cpu/0/power/0", "Package", "power")
    ];

    [Fact]
    [Trait("Category", "Unit")]
    public void RepresentativeDetailPublishesNoPerCoreSensors()
    {
        CatalogNormalizer.Build(AmdCores).Sensors.ShouldAllBe(sensor => sensor.Scope == "package");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PerCoreDetailAddsLoadAndClockForEachCoreAndSkipsThreadsAndFactors()
    {
        var catalog = CatalogNormalizer.Build(AmdCores, CatalogNormalizer.DetailPerCore);

        var coreSensors = catalog.Sensors.Where(sensor => sensor.Scope == "core").ToArray();
        coreSensors.Select(sensor => sensor.Id).ShouldBe(
            ["cpu.core.1.load", "cpu.core.2.load", "cpu.core.1.clock", "cpu.core.2.clock"]);
        coreSensors.ShouldAllBe(sensor => sensor.ScopeRef == sensor.Id.Split('.')[2]);
        coreSensors.Where(sensor => sensor.Metric == "clock").ShouldAllBe(sensor => sensor.Quality == "substitute");
        catalog.Sensors.Count(sensor => sensor.Scope == "package").ShouldBe(4); // temp, load, power, clock
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PerCoreValuesFollowTheirOwnRawSensorAndKeepZeroLoadButNotZeroClock()
    {
        var catalog = CatalogNormalizer.Build(AmdCores, CatalogNormalizer.DetailPerCore);

        var values = catalog.Map([
            Ok("/cpu/0/temperature/0", 55f),
            Ok("/cpu/0/load/0", 10f),
            Ok("/cpu/0/load/1", 0f),
            Ok("/cpu/0/load/2", 33f),
            Ok("/cpu/0/clock/1", 0f),
            Ok("/cpu/0/clock/2", 3700f),
            Ok("/cpu/0/power/0", 20f)
        ]).ToDictionary(value => value.SensorId);

        values["cpu.core.1.load"].ShouldBe(new SampleValue("cpu.core.1.load", 0, "ok")); // an idle core is 0 %
        values["cpu.core.2.load"].Number.ShouldBe(33);
        values["cpu.core.1.clock"].ShouldBe(new SampleValue("cpu.core.1.clock", null, "invalid")); // 0 MHz is not a reading
        values["cpu.core.2.clock"].Number.ShouldBe(3700);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void HybridCoreNamesKeepTheirKindInTheKey()
    {
        var catalog = CatalogNormalizer.Build(
            [
                Raw("/cpu/0/temperature/1", "P-Core #1", "temperature"),
                Raw("/cpu/0/temperature/9", "E-Core #9", "temperature"),
                Raw("/cpu/0/temperature/20", "LPE-Core #1", "temperature")
            ],
            CatalogNormalizer.DetailPerCore);

        catalog.Sensors.Where(sensor => sensor.Scope == "core").Select(sensor => sensor.ScopeRef)
            .ShouldBe(["p1", "e9", "lpe1"]);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PerCoreSensorsAreMissingWhenTheRawReadingIsAbsent()
    {
        var catalog = CatalogNormalizer.Build(AmdCores, CatalogNormalizer.DetailPerCore);

        var values = catalog.Map([]).ToDictionary(value => value.SensorId);

        values["cpu.core.1.load"].ShouldBe(new SampleValue("cpu.core.1.load", null, "missing"));
    }
}
