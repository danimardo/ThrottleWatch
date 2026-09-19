namespace ThrottleWatch.SensorAgent.Tests;

using Shouldly;

public sealed class FoundationTests
{
    [Fact]
    [Trait("Category", "Unit")]
    public void TestProjectIsOperational()
    {
        true.ShouldBeTrue();
    }

    [Fact]
    [Trait("Category", "Integration")]
    public void IntegrationCategoryIsAvailable()
    {
        "integration".ShouldNotBeNullOrWhiteSpace();
    }

    [Fact]
    [Trait("Category", "Protocol")]
    public void ProtocolCategoryIsAvailable()
    {
        "protocol".ShouldNotBeNullOrWhiteSpace();
    }
}
