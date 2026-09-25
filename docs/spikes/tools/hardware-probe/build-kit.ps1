<#
.SYNOPSIS
  Builds the portable probe kit. Runs on the development machine, not on the machine to probe.

.DESCRIPTION
  Publishes the sidecar self-contained (the target machine needs no .NET installed) and copies
  the two scripts and the README next to it. Output goes to `dist\` and is zipped into
  `throttlewatch-probe-kit.zip`, ready for a USB stick or an upload to the rented machine.

  `dist\` and the .zip are build artefacts and are not versioned.

  ASCII only on purpose: Windows PowerShell 5.1 reads a UTF-8 file without BOM as ANSI and
  mis-parses accented characters.

.EXAMPLE
  .\build-kit.ps1
#>
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
# hardware-probe -> tools -> spikes -> docs -> repository root
$repo = (Resolve-Path (Join-Path $root '..\..\..\..')).Path
$dist = Join-Path $root 'dist'

if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Write-Output "Publishing the self-contained sidecar (win-x64)..."
& dotnet publish (Join-Path $repo 'apps\sensor-agent\SensorAgent.csproj') `
  -c Release -r win-x64 --self-contained true `
  -o $dist --nologo
if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed with code $LASTEXITCODE" }

Copy-Item (Join-Path $root 'run-probe.ps1') $dist -Force
Copy-Item (Join-Path $root 'README.md') $dist -Force
Copy-Item (Join-Path $root '..\sensor-access-matrix.ps1') $dist -Force

$zip = Join-Path $root 'throttlewatch-probe-kit.zip'
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$dist\*" -DestinationPath $zip

$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Output ""
Write-Output ("Kit ready: " + $zip + " (" + $size + " MB)")
Write-Output "Copy the whole thing to the target machine and run run-probe.ps1 there."
