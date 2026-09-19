using System.Diagnostics;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

// Usage: GlassHost <url> <seconds> <output-file>
// Loads the bench URL in WebView2, waits until the page publishes its result
// (#result), prints one JSON line with the page result plus CPU time consumed by
// every process of this WebView2 instance during the window, then exits.
internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
    var url = args.Length > 0 ? args[0] : "http://localhost:4179/?glass=full";
    var seconds = args.Length > 1 ? int.Parse(args[1]) : 20;
    var output = args.Length > 2 ? args[2] : Path.Combine(Path.GetTempPath(), "glass-bench-result.json");
    var udf = Path.Combine(Path.GetTempPath(), "throttlewatch-glass-bench-udf");

    ApplicationConfiguration.Initialize();
    var form = new Form { Width = 1100, Height = 760, Text = "T019c glass bench (WebView2)" };
    var view = new WebView2 { Dock = DockStyle.Fill };
    form.Controls.Add(view);

    form.Shown += async (_, _) =>
    {
      try
      {
        var env = await CoreWebView2Environment.CreateAsync(null, udf, new CoreWebView2EnvironmentOptions());
        await view.EnsureCoreWebView2Async(env);
        var browserPid = (int)view.CoreWebView2.BrowserProcessId;
        view.CoreWebView2.Navigate(url);
        await Task.Delay(3000); // matches the page's default delay=3000 so both windows line up
        var before = CpuTimes(browserPid);
        var stopwatch = Stopwatch.StartNew();
        string result = "";
        while (stopwatch.Elapsed.TotalSeconds < seconds + 10)
        {
            await Task.Delay(500);
            var raw = await view.CoreWebView2.ExecuteScriptAsync("document.getElementById('result').textContent");
            var text = System.Text.Json.JsonSerializer.Deserialize<string>(raw) ?? "";
            if (text.Length > 0) { result = text; break; }
        }
        var elapsed = stopwatch.Elapsed.TotalSeconds;
        var after = CpuTimes(browserPid);
        var cpuMs = after.Sum(p => p.Value) - before.Sum(p => before.TryGetValue(p.Key, out var b) ? b : 0);
        var logical = Environment.ProcessorCount;
        var cpuPct = cpuMs / (elapsed * 1000.0) / logical * 100.0;
        File.WriteAllText(output, System.Text.Json.JsonSerializer.Serialize(new
        {
            page = System.Text.Json.JsonDocument.Parse(result.Length > 0 ? result : "{}").RootElement,
            webview2_version = env.BrowserVersionString,
            processes = after.Count,
            window_s = Math.Round(elapsed, 1),
            cpu_ms = Math.Round(cpuMs, 0),
            cpu_pct_of_machine = Math.Round(cpuPct, 3),
            cpu_pct_of_one_core = Math.Round(cpuMs / (elapsed * 1000.0) * 100.0, 2),
            logical_processors = logical
        }));
      }
      catch (Exception exception)
      {
        File.WriteAllText(output, System.Text.Json.JsonSerializer.Serialize(new { error = exception.ToString() }));
      }
      Application.Exit();
    };

    Application.Run(form);
    }

    private static Dictionary<int, double> CpuTimes(int browserPid)
{
    // Every process of this WebView2 instance descends from the browser process.
    var all = Process.GetProcessesByName("msedgewebview2");
    var parents = new Dictionary<int, int>();
    foreach (var p in all)
    {
        try { parents[p.Id] = ParentPid(p.Id); } catch { }
    }
    var mine = new Dictionary<int, double>();
    foreach (var p in all)
    {
        var id = p.Id;
        var belongs = id == browserPid;
        var cursor = id;
        for (var hops = 0; !belongs && hops < 8 && parents.TryGetValue(cursor, out var parent); hops++)
        {
            if (parent == browserPid) belongs = true;
            cursor = parent;
        }
        if (!belongs) continue;
        try { mine[id] = p.TotalProcessorTime.TotalMilliseconds; } catch { }
    }
    return mine;
}

    private static int ParentPid(int pid)
{
    using var search = new System.Management.ManagementObjectSearcher($"SELECT ParentProcessId FROM Win32_Process WHERE ProcessId = {pid}");
    foreach (var item in search.Get()) return Convert.ToInt32(item["ParentProcessId"]);
    return -1;
}
}
