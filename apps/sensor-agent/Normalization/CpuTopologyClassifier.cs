namespace ThrottleWatch.SensorAgent.Normalization;

using System.Runtime.Intrinsics.X86;
using System.Numerics;
using System.Runtime.InteropServices;

public enum CpuGroupKind
{
    P,
    E,
    LpE,
    Homogeneous,
    Unknown
}

public sealed record LogicalProcessorInfo(int LogicalId, int? EfficiencyClass, int? IntelHybridClass);

public static class CpuTopologyClassifier
{
    private const int HybridLeaf = 0x1A;

    public static CpuGroupKind Classify(string vendor, IReadOnlyList<LogicalProcessorInfo> processors)
    {
        if (processors.Count == 0)
        {
            return CpuGroupKind.Unknown;
        }

        if (vendor.Equals("amd", StringComparison.OrdinalIgnoreCase))
        {
            return CpuGroupKind.Homogeneous;
        }

        if (!vendor.Equals("intel", StringComparison.OrdinalIgnoreCase))
        {
            return CpuGroupKind.Unknown;
        }

        var classes = processors
            .Select(processor => processor.IntelHybridClass ?? processor.EfficiencyClass)
            .ToArray();
        if (classes.Any(static value => value is null))
        {
            return CpuGroupKind.Unknown;
        }

        return classes.All(static value => value == 0)
            ? CpuGroupKind.P
            : classes.All(static value => value is > 0)
                ? CpuGroupKind.E
                : CpuGroupKind.Unknown;
    }

    public static IReadOnlyDictionary<CpuGroupKind, IReadOnlyList<int>> GroupLogicalProcessors(
        string vendor,
        IReadOnlyList<LogicalProcessorInfo> processors)
    {
        if (!vendor.Equals("intel", StringComparison.OrdinalIgnoreCase))
        {
            return new Dictionary<CpuGroupKind, IReadOnlyList<int>>
            {
                [CpuGroupKind.Homogeneous] = processors.Select(static processor => processor.LogicalId).ToArray()
            };
        }

        var groups = processors
            .GroupBy(processor => processor.IntelHybridClass ?? processor.EfficiencyClass)
            .Where(static group => group.Key is not null)
            .ToDictionary(
                static group => group.Key switch
                {
                    0 => CpuGroupKind.P,
                    2 => CpuGroupKind.LpE,
                    _ => CpuGroupKind.E
                },
                static group => (IReadOnlyList<int>)group.Select(static processor => processor.LogicalId).ToArray());
        return groups.Count == 0 ? new Dictionary<CpuGroupKind, IReadOnlyList<int>>() : groups;
    }

    public static IReadOnlyList<LogicalProcessorInfo> ReadLogicalProcessorInfo(string vendor)
    {
        var processorCount = Math.Max(ReadWindowsLogicalProcessorCount() ?? Environment.ProcessorCount, 1);
        var result = new List<LogicalProcessorInfo>(processorCount);
        for (var logicalId = 0; logicalId < processorCount; logicalId++)
        {
            var hybridClass = ReadIntelHybridClass(vendor);
            result.Add(new LogicalProcessorInfo(logicalId, null, hybridClass));
        }
        return result;
    }

    public static IReadOnlyDictionary<CpuGroupKind, IReadOnlyList<int>> ReadGroups(string vendor)
        => GroupLogicalProcessors(vendor, ReadLogicalProcessorInfo(vendor));

    private static int? ReadIntelHybridClass(string vendor)
    {
        if (!vendor.Equals("intel", StringComparison.OrdinalIgnoreCase) || !X86Base.IsSupported)
        {
            return null;
        }

        try
        {
            var result = X86Base.CpuId(HybridLeaf, 0);
            var coreType = (result.Eax >> 24) & 0xFF;
            return coreType switch
            {
                0x40 => 0,
                0x20 => 1,
                _ => null
            };
        }
        catch (PlatformNotSupportedException)
        {
            return null;
        }
    }

    private static int? ReadWindowsLogicalProcessorCount()
    {
        if (!OperatingSystem.IsWindows()) return null;
        uint length = 0;
        _ = GetLogicalProcessorInformationEx(0, IntPtr.Zero, ref length);
        if (length == 0) return null;
        var buffer = Marshal.AllocHGlobal((int)length);
        try
        {
            if (!GetLogicalProcessorInformationEx(0, buffer, ref length)) return null;
            var offset = 0;
            var total = 0;
            while (offset + 16 <= length)
            {
                var size = Marshal.ReadInt32(buffer, offset + 4);
                if (size < 16 || offset + size > length) break;
                var relationship = Marshal.ReadInt32(buffer, offset);
                if (relationship == 0)
                {
                    var groupCount = Marshal.ReadInt16(buffer, offset + 12);
                    var affinityOffset = offset + 16;
                    for (var group = 0; group < groupCount; group++)
                    {
                        var mask = IntPtr.Size == 8
                            ? unchecked((ulong)Marshal.ReadInt64(buffer, affinityOffset))
                            : unchecked((uint)Marshal.ReadInt32(buffer, affinityOffset));
                        total += BitOperations.PopCount(mask);
                        affinityOffset += IntPtr.Size == 8 ? 16 : 12;
                    }
                }
                offset += size;
            }
            return total > 0 ? total : null;
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetLogicalProcessorInformationEx(
        int relationshipType,
        IntPtr buffer,
        ref uint returnedLength);
}
