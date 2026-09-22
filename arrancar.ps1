<#
.SINOPSIS
    Arranca ThrottleWatch (el binario ya compilado), cerrando antes cualquier
    instancia que haya quedado zombie o mal cerrada (de la app o del sidecar
    del colector, SensorAgent.exe).

.DESCRIPCION
    No recompila nada: usa el .exe que ya exista en
    apps\desktop\src-tauri\target\debug\throttlewatch.exe (con la feature
    `custom-protocol`, que incrusta el frontend). Si no existe, avisa con el
    comando para generarlo.

.EJEMPLO
    .\arrancar.ps1
#>

$ErrorActionPreference = 'Stop'

$exePath = Join-Path $PSScriptRoot 'apps\desktop\src-tauri\target\debug\throttlewatch.exe'

if (-not (Test-Path $exePath)) {
    Write-Error @"
No se encuentra el ejecutable en:
  $exePath

Compílalo primero desde apps\desktop:
  pnpm exec vite build
  cargo build --locked --features custom-protocol --manifest-path src-tauri\Cargo.toml
"@
    exit 1
}

function Stop-Zombie([string]$processName) {
    $procesos = Get-Process -Name $processName -ErrorAction SilentlyContinue
    if (-not $procesos) {
        return
    }
    $ids = ($procesos | ForEach-Object { $_.Id }) -join ', '
    Write-Output "$processName ya estaba en ejecucion (PID $ids) - cerrandola antes de arrancar de nuevo..."
    $procesos | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
    $siguenAhi = Get-Process -Name $processName -ErrorAction SilentlyContinue
    if ($siguenAhi) {
        $idsRestantes = ($siguenAhi | ForEach-Object { $_.Id }) -join ', '
        Write-Warning "$processName no se pudo cerrar del todo (PID $idsRestantes seguia vivo tras el intento)."
    }
}

# La app primero: si sigue viva, su propio supervisor del colector relanza
# SensorAgent.exe en cuanto lo mata (comprobado: cerrar el sidecar con la app
# todavia viva solo hace que la app cree uno nuevo). Con la app ya muerta no
# queda nadie que lo resucite, y cualquier SensorAgent.exe que quedara suelto
# se cierra limpio.
Stop-Zombie 'throttlewatch'
Stop-Zombie 'SensorAgent'

Write-Output "Arrancando ThrottleWatch ($exePath)..."
$proceso = Start-Process -FilePath $exePath -PassThru
Start-Sleep -Seconds 2

if (Get-Process -Id $proceso.Id -ErrorAction SilentlyContinue) {
    Write-Output "ThrottleWatch arrancada (PID $($proceso.Id))."
} else {
    Write-Error "ThrottleWatch se cerro justo despues de arrancar; revisa los registros en %LOCALAPPDATA%\ThrottleWatch\logs."
    exit 1
}
