<#
.SYNOPSIS
  Collects, on a machine that is not the development one, everything needed to add a row to the
  hardware matrix (T019, T109) and to decide T028b2 on AMD.

.DESCRIPTION
  Installs nothing, touches no registry key and changes no setting: it only reads, and writes a
  report into its own folder. Works without PawnIO and without elevation, though it then only
  reaches tier B/C and says so in the report.

  Leaves an `informe-<cpu>-<date>` folder and its .zip next to this script. That .zip is what
  must be sent back.

  ASCII only on purpose: Windows PowerShell 5.1 reads a UTF-8 file without BOM as ANSI and
  mis-parses accented characters. Accented prose lives in README.md, which nothing parses.

.PARAMETER Seconds
  Seconds of per-logical-processor counters. 10 by default; raise to 60 to watch behaviour under
  a load you start yourself in parallel.

.EXAMPLE
  .\run-probe.ps1
.EXAMPLE
  .\run-probe.ps1 -Seconds 60
#>
param(
  [int] $Seconds = 10
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$sidecar = Join-Path $root 'SensorAgent.exe'

if (-not (Test-Path $sidecar)) {
  throw "SensorAgent.exe not found next to this script. Use the full kit, not just the .ps1."
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$os = Get-CimInstance Win32_OperatingSystem
$system = Get-CimInstance Win32_ComputerSystem
$pawnio = Get-Service PawnIO -ErrorAction SilentlyContinue

$slug = ($cpu.Name -replace '\(R\)|\(TM\)|CPU|Processor|@.*', '' -replace '[^A-Za-z0-9]+', '-').Trim('-').ToLower()
$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')
$outDir = Join-Path $root "informe-$slug-$stamp"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Write-Output ""
Write-Output "ThrottleWatch - hardware probe"
Write-Output ("  CPU      : " + $cpu.Name.Trim())
Write-Output ("  Elevated : " + $elevated)
Write-Output ("  PawnIO   : " + $(if ($pawnio) { $pawnio.Status } else { 'not installed' }))
Write-Output ("  Output   : " + $outDir)
Write-Output ""

if (-not $elevated) {
  Write-Warning "Session NOT elevated: tier A is unreachable and the AMD PM table cannot be read."
  Write-Warning "The report is still useful as a tier B/C row, but T028b2 needs elevation."
}
if (-not $pawnio) {
  Write-Warning "PawnIO is not installed: no low-level access. The report will record it that way."
}

# 1) Sensor matrix row: reuses the T019 script as-is, so its counter logic is not duplicated
#    (it dodges localized PDH names by using the formatted WMI class).
$phase = if ($pawnio) { 'installed' } else { 'before' }
Write-Output ("[1/3] Sensor matrix and counters (" + $Seconds + " s)...")
& (Join-Path $root 'sensor-access-matrix.ps1') -Sidecar $sidecar -Phase $phase -CounterSeconds $Seconds
Get-ChildItem -Path $root -Filter "sensor-access-$phase.json" | Move-Item -Destination $outDir -Force

# 2) Raw PM table dump (AMD only; on Intel it answers NOT_AMD and nothing breaks).
Write-Output "[2/3] SMU PM table dump..."
& $sidecar --dump-pm-table 2>&1 | Out-File (Join-Path $outDir 'pm-table-dump.json') -Encoding utf8

# 3) Readable summary, to tell at a glance whether the report is usable.
Write-Output "[3/3] Summary..."
$probe = $null
$dump = $null
try { $probe = Get-Content (Join-Path $outDir "sensor-access-$phase.json") -Raw | ConvertFrom-Json } catch { }
try { $dump = Get-Content (Join-Path $outDir 'pm-table-dump.json') -Raw | ConvertFrom-Json } catch { }

$lines = @(
  'ThrottleWatch - hardware probe',
  '==============================',
  ('Captured (UTC) : ' + (Get-Date).ToUniversalTime().ToString('o')),
  ('CPU            : ' + $cpu.Name.Trim()),
  ('Identification : ' + $cpu.Description),
  ('Cores          : ' + $cpu.NumberOfCores + ' physical / ' + $cpu.NumberOfLogicalProcessors + ' logical'),
  ('Windows        : ' + $os.Caption + ' ' + $os.Version),
  ('Hypervisor     : ' + $system.HypervisorPresent),
  ('Elevated       : ' + $elevated),
  ('PawnIO         : ' + $(if ($pawnio) { "$($pawnio.Status)/$($pawnio.StartType)" } else { 'not installed' })),
  '',
  'Low-level access (probe)',
  ('  state        : ' + $probe.probe.state),
  ('  provider     : ' + $probe.probe.provider + ' ' + $probe.probe.provider_version),
  ('  details      : ' + $probe.probe.details_code),
  '',
  'SMU PM table (AMD only)',
  ('  result       : ' + $dump.details_code),
  ('  SMU version  : ' + $dump.smu_version),
  ('  table version: ' + $dump.table_version),
  ('  size (bytes) : ' + $dump.table_size_bytes),
  ('  entries      : ' + $dump.entry_count),
  '',
  'Send back the whole folder (or its .zip).',
  'It holds no serial numbers, user names or paths.'
)

$lines | Out-File (Join-Path $outDir 'RESUMEN.txt') -Encoding utf8
Write-Output ""
$lines | ForEach-Object { Write-Output $_ }

$zip = "$outDir.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$outDir\*" -DestinationPath $zip
Write-Output ""
Write-Output "Done. Send this file back:"
Write-Output ("  " + $zip)
