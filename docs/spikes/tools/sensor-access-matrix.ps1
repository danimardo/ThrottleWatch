<#
.SYNOPSIS
  T019: collects the evidence for one row of the hardware matrix in docs/spikes/sensor-access.md.
.DESCRIPTION
  Runs unelevated. Writes sensor-access-<Phase>.json next to this script with machine facts
  (no serial numbers, user names or paths), the low-level probe output, 10 s of PDH counters
  per logical processor and the LibreHardwareMonitor catalog reported by the sidecar test.
.PARAMETER Sidecar
  Path to SensorAgent.exe (Debug build or self-contained publish).
.PARAMETER Phase
  before | installed | after-reboot
#>
param(
  [Parameter(Mandatory = $true)] [string] $Sidecar,
  [Parameter(Mandatory = $true)] [ValidateSet('before', 'installed', 'after-reboot')] [string] $Phase,
  [int] $CounterSeconds = 10
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Sidecar)) { throw "sidecar not found: $Sidecar" }

$identity = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$os = Get-CimInstance Win32_OperatingSystem
$battery = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue
$service = Get-Service PawnIO -ErrorAction SilentlyContinue
$uninstall = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO' -ErrorAction SilentlyContinue

$probeRaw = & $Sidecar --probe-low-level 2>&1 | Out-String
$probe = $null
try { $probe = $probeRaw | ConvertFrom-Json } catch { $probe = @{ raw = $probeRaw } }

# PDH counter names are localized (on a Spanish Windows the English path is "not found" and
# PdhLookupPerfNameByIndex rejects the v2 "Processor Information" indexes). The formatted WMI
# class keeps English names on every locale, so it is used here; the Rust host must use
# PdhAddEnglishCounter for the same reason (see sensor-access.md).
$counters = for ($n = 0; $n -lt $CounterSeconds; $n += 1) {
  $rows = Get-CimInstance Win32_PerfFormattedData_Counters_ProcessorInformation |
    Where-Object { $_.Name -notlike '*_Total' } |
    ForEach-Object {
      [pscustomobject]@{
        cpu                     = $_.Name
        percent_performance     = [int]$_.PercentProcessorPerformance
        frequency_mhz           = [int]$_.ProcessorFrequency
        percent_utility         = [int]$_.PercentProcessorUtility
        percent_time            = [int]$_.PercentProcessorTime
      }
    }
  [pscustomobject]@{ timestamp = (Get-Date).ToString('o'); values = @($rows) }
  Start-Sleep -Seconds 1
}
$englishPaths = @('\Processor Information(*)\% Processor Performance', '\Processor Information(*)\Processor Frequency', '\Processor Information(*)\% Processor Utility', '\Processor Information(*)\% Processor Time')

$report = [pscustomobject]@{
  phase               = $Phase
  captured_at_utc     = (Get-Date).ToUniversalTime().ToString('o')
  elevated            = $identity.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  cpu_name            = $cpu.Name.Trim()
  cpu_family_model    = "family=$($cpu.Family) caption=$($cpu.Caption)"
  logical_processors  = $cpu.NumberOfLogicalProcessors
  physical_cores      = $cpu.NumberOfCores
  max_clock_mhz       = $cpu.MaxClockSpeed
  windows_build       = "$($os.Caption) $($os.Version)"
  hypervisor_present  = (Get-CimInstance Win32_ComputerSystem).HypervisorPresent
  power               = if ($battery) { "battery status=$($battery.BatteryStatus) charge=$($battery.EstimatedChargeRemaining)%" } else { 'AC (no battery detected)' }
  power_plan          = (powercfg /getactivescheme) -replace '.*\((.*)\).*', '$1'
  pawnio_service      = if ($service) { "$($service.Status)/$($service.StartType)" } else { 'not registered' }
  pawnio_version      = if ($uninstall) { $uninstall.DisplayVersion } else { $null }
  probe               = $probe
  counter_source      = 'Win32_PerfFormattedData_Counters_ProcessorInformation'
  counter_paths_en    = $englishPaths
  counters            = $counters
}

$out = Join-Path $PSScriptRoot "sensor-access-$Phase.json"
$report | ConvertTo-Json -Depth 8 | Out-File $out -Encoding utf8
Write-Output "written $out"
Write-Output ("probe: state={0} provider={1} details={2}" -f $probe.state, $probe.provider_version, $probe.details_code)
if ($probe.aperf_mperf_ratio) {
  Write-Output ("probe: aperf_mperf_ratio={0} window_ms={1}" -f $probe.aperf_mperf_ratio, $probe.aperf_mperf_window_ms)
}
