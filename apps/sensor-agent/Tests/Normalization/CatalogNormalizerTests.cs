using System.Linq;
using Shouldly;
using ThrottleWatch.SensorAgent.Collector;
using ThrottleWatch.SensorAgent.Normalization;

namespace ThrottleWatch.SensorAgent.Tests.Normalization;

public sealed class CatalogNormalizerTests
{
    private static SensorDescriptor Raw(string name, string metric) =>
        new($"/cpu/0/{name}", "/cpu/0", name, metric, metric == "temperature" ? "celsius" : "unknown");

    private static SensorReading Ok(string name, float value) => new($"/cpu/0/{name}", value, "ok");

    private static SensorReading Bad(string name, string status) => new($"/cpu/0/{name}", null, status);

    [Theory]
    [InlineData("CPU Package", "package")]
    [InlineData("Core (Tctl/Tdie)", "tctl")]
    [InlineData("Tctl", "tctl")]
    [InlineData("CCD1 (Tdie)", "tdie")]
    [InlineData("Core #3", "core")]
    [InlineData("Core Max", "core")]
    [InlineData("GPU Hot Spot", null)]
    [Trait("Category", "Unit")]
    public void InfersTheScopeClassOfATemperatureFromItsName(string name, string? expected)
    {
        CatalogNormalizer.InferTemperatureScope(name).ShouldBe(expected);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void AmdLikeHostPublishesOneSensorPerMagnitudeWithTctlAsSubstitute()
    {
        var catalog = CatalogNormalizer.Build([
            Raw("Core (Tctl/Tdie)", "temperature"),
            Raw("CPU Total", "load"),
            Raw("Package", "power"),
            Raw("Core #1", "clock"),
            Raw("Core #2", "clock"),
            Raw("Bus Speed", "clock")
        ]);

        catalog.Sensors.Select(sensor => sensor.Id).ShouldBe(
            [CatalogNormalizer.TemperatureId, CatalogNormalizer.LoadId, CatalogNormalizer.PowerId, CatalogNormalizer.ClockId]);
        catalog.Sensors.First(sensor => sensor.Id == CatalogNormalizer.TemperatureId).Quality.ShouldBe("substitute");
        catalog.Sensors.First(sensor => sensor.Id == CatalogNormalizer.ClockId).Quality.ShouldBe("substitute");

        var values = catalog.Map([
            Ok("Core (Tctl/Tdie)", 61.5f),
            Ok("CPU Total", 12f),
            Ok("Package", 34.25f),
            Ok("Core #1", 3600f),
            Ok("Core #2", 3925f),
            Ok("Bus Speed", 100f)
        ]).ToDictionary(value => value.SensorId);

        values[CatalogNormalizer.TemperatureId].Number.ShouldBe(61.5);
        values[CatalogNormalizer.LoadId].Number.ShouldBe(12);
        values[CatalogNormalizer.PowerId].Number.ShouldBe(34.25);
        values[CatalogNormalizer.ClockId].Number.ShouldBe(3925); // highest core clock, not the bus
        values.Values.ShouldAllBe(value => value.Status == "ok");
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void PackageTemperatureWinsAndIsDirect()
    {
        var catalog = CatalogNormalizer.Build([Raw("Core #1", "temperature"), Raw("CPU Package", "temperature"), Raw("Core (Tctl/Tdie)", "temperature")]);

        catalog.Sensors.Single().Quality.ShouldBe("direct");
        catalog.Map([Ok("Core #1", 80f), Ok("CPU Package", 70f), Ok("Core (Tctl/Tdie)", 90f)])
            .Single().Number.ShouldBe(70);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void WithoutPackageTheHottestCoreIsChosenOnEverySample()
    {
        var catalog = CatalogNormalizer.Build([Raw("Core #1", "temperature"), Raw("Core #2", "temperature")]);

        catalog.Map([Ok("Core #1", 60f), Ok("Core #2", 71f)]).Single().Number.ShouldBe(71);
        catalog.Map([Ok("Core #1", 75f), Ok("Core #2", 71f)]).Single().Number.ShouldBe(75);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void AbsentReadingsStayAbsentInsteadOfBecomingZero()
    {
        var catalog = CatalogNormalizer.Build([Raw("Core (Tctl/Tdie)", "temperature"), Raw("CPU Total", "load"), Raw("Package", "power"), Raw("Core #1", "clock")]);

        var values = catalog.Map([Bad("Core (Tctl/Tdie)", "invalid"), Bad("CPU Total", "missing"), Bad("Package", "invalid"), Bad("Core #1", "invalid")])
            .ToDictionary(value => value.SensorId);

        values[CatalogNormalizer.TemperatureId].ShouldBe(new SampleValue(CatalogNormalizer.TemperatureId, null, "missing"));
        values[CatalogNormalizer.LoadId].ShouldBe(new SampleValue(CatalogNormalizer.LoadId, null, "missing"));
        values[CatalogNormalizer.PowerId].ShouldBe(new SampleValue(CatalogNormalizer.PowerId, null, "invalid"));
        values[CatalogNormalizer.ClockId].ShouldBe(new SampleValue(CatalogNormalizer.ClockId, null, "missing"));
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void HostWithOnlyLoadPublishesOnlyLoad()
    {
        // A virtualized guest: the collector already hides temperature, power and voltage.
        var catalog = CatalogNormalizer.Build([Raw("CPU Total", "load"), Raw("CPU Core #1", "load")]);

        catalog.Sensors.Select(sensor => sensor.Id).ShouldBe([CatalogNormalizer.LoadId]);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void EmptyCatalogPublishesNothing()
    {
        CatalogNormalizer.Build([]).Sensors.ShouldBeEmpty();
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void ZeroFromAnUnreadableSensorIsInvalidExceptForLoad()
    {
        // An AMD CPU without low-level access: LibreHardwareMonitorLib reports 0 °C, 0 W and 0 MHz as "ok".
        var catalog = CatalogNormalizer.Build([Raw("Core (Tctl/Tdie)", "temperature"), Raw("CPU Total", "load"), Raw("Package", "power"), Raw("Core #1", "clock")]);

        var values = catalog.Map([Ok("Core (Tctl/Tdie)", 0f), Ok("CPU Total", 0f), Ok("Package", 0f), Ok("Core #1", 0f)])
            .ToDictionary(value => value.SensorId);

        values[CatalogNormalizer.TemperatureId].ShouldBe(new SampleValue(CatalogNormalizer.TemperatureId, null, "invalid"));
        values[CatalogNormalizer.PowerId].ShouldBe(new SampleValue(CatalogNormalizer.PowerId, null, "invalid"));
        values[CatalogNormalizer.ClockId].ShouldBe(new SampleValue(CatalogNormalizer.ClockId, null, "invalid"));
        values[CatalogNormalizer.LoadId].ShouldBe(new SampleValue(CatalogNormalizer.LoadId, 0, "ok")); // idle is a real 0 %
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void AZeroCoreDoesNotHideTheOthers()
    {
        var catalog = CatalogNormalizer.Build([Raw("Core #1", "temperature"), Raw("Core #2", "temperature"), Raw("Core #1", "clock"), Raw("Core #2", "clock")]);

        var values = catalog.Map([Ok("Core #1", 0f), Ok("Core #2", 55f)]).ToDictionary(value => value.SensorId);

        values[CatalogNormalizer.TemperatureId].Number.ShouldBe(55);
    }

    [Fact]
    [Trait("Category", "Unit")]
    public void DuplicateRawIdsDoNotBreakTheMapping()
    {
        var catalog = CatalogNormalizer.Build([Raw("CPU Total", "load")]);

        catalog.Map([Ok("CPU Total", 5f), Ok("CPU Total", 9f)]).Single().Number.ShouldBe(5);
    }
}
