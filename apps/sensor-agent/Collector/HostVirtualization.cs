using System.Runtime.Intrinsics.X86;
using Microsoft.Win32;

namespace ThrottleWatch.SensorAgent.Collector;

/// <summary>
/// Tells whether the sidecar runs inside a virtual machine. A guest has no physical sensors:
/// LibreHardwareMonitorLib still reports temperature, power and voltage there, but the values come
/// from virtual MSRs (observed on a virtualized AMD CI runner: 0 °C, 0 W and a constant 1.55 V, all
/// with status "ok"). Such readings must never reach the engine, so the collector drops them.
/// </summary>
public static class HostVirtualization
{
    // CPUID leaf 1, ECX bit 31 "hypervisor present" is also set on bare-metal Windows hosts with
    // virtualization-based security, so it is only half of the test: the SMBIOS product name of a
    // guest names the hypervisor.
    private static readonly string[] VirtualProductNames =
        ["Virtual Machine", "VMware", "VirtualBox", "QEMU", "KVM", "Google Compute Engine"];

    public static bool IsVirtualized() => Classify(HypervisorBitSet(), SystemProductName());

    public static bool Classify(bool hypervisorBit, string productName) =>
        hypervisorBit
        && VirtualProductNames.Any(name => productName.Contains(name, StringComparison.OrdinalIgnoreCase));

    private static bool HypervisorBitSet() => X86Base.IsSupported && (X86Base.CpuId(1, 0).Ecx & (1 << 31)) != 0;

    private static string SystemProductName() =>
        Registry.GetValue(@"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\BIOS", "SystemProductName", null) as string
        ?? string.Empty;
}
