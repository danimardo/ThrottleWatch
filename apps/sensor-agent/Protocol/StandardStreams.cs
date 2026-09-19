namespace ThrottleWatch.SensorAgent.Protocol;

using System.Text;

internal sealed class StandardStreams : IDisposable
{
    private readonly StreamReader input;
    private readonly StreamWriter output;

    public StandardStreams()
    {
#pragma warning disable RS0030
        input = new StreamReader(Console.OpenStandardInput(), Encoding.UTF8);
        output = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false))
        {
            AutoFlush = true
        };
#pragma warning restore RS0030
    }

    public ValueTask<string?> ReadLineAsync(CancellationToken cancellationToken)
    {
        return input.ReadLineAsync(cancellationToken);
    }

    public Task WriteLineAsync(string line)
    {
        return output.WriteLineAsync(line);
    }

    public void Dispose()
    {
        input.Dispose();
        output.Dispose();
    }
}
