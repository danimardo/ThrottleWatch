<#
.SINOPSIS
    Arranca ThrottleWatch (el binario ya compilado), cerrando antes cualquier
    instancia que haya quedado zombie o mal cerrada (de la app o del sidecar
    del colector, SensorAgent.exe).

.DESCRIPCION
    Usa el .exe en apps\desktop\src-tauri\target\debug\throttlewatch.exe (con
    la feature `custom-protocol`, que incrusta el frontend). Si no existe,
    avisa con el comando para generarlo. Si existe pero es mas antiguo que el
    codigo fuente (frontend, Rust o sistema de diseno), lo recompila antes de
    arrancar — sin este chequeo, `arrancar.ps1` seguia lanzando una version
    vieja del binario aunque el codigo hubiera cambiado, sin ningun aviso.

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

# Compara el binario contra el codigo fuente que lo produce: si hay una edicion mas reciente que
# el .exe, arrancarlo tal cual mostraria una version vieja sin ningun aviso (lo que paso el
# 2026-09-28: varios arreglos de UI no se veian porque este script solo relanzaba el binario ya
# compilado). Se compara despues de matar zombies, no antes: un throttlewatch.exe todavia vivo
# bloquea el propio fichero que `cargo build` necesita sobrescribir.
function Get-LatestSourceWriteUtc {
    $rutas = @(
        (Join-Path $PSScriptRoot 'apps\desktop\src'),
        (Join-Path $PSScriptRoot 'apps\desktop\src-tauri\src'),
        (Join-Path $PSScriptRoot 'apps\desktop\src-tauri\build.rs'),
        (Join-Path $PSScriptRoot 'apps\desktop\src-tauri\Cargo.toml'),
        (Join-Path $PSScriptRoot 'apps\desktop\src-tauri\Cargo.lock'),
        (Join-Path $PSScriptRoot 'apps\desktop\package.json'),
        (Join-Path $PSScriptRoot 'apps\desktop\vite.config.ts'),
        (Join-Path $PSScriptRoot 'design\components')
    )
    $masReciente = $null
    foreach ($ruta in $rutas) {
        if (-not (Test-Path $ruta)) { continue }
        $item = Get-Item $ruta
        $fechas = if ($item.PSIsContainer) {
            (Get-ChildItem $ruta -Recurse -File -ErrorAction SilentlyContinue).LastWriteTimeUtc
        } else {
            @($item.LastWriteTimeUtc)
        }
        foreach ($fecha in $fechas) {
            if (-not $masReciente -or $fecha -gt $masReciente) { $masReciente = $fecha }
        }
    }
    return $masReciente
}

$fuenteReciente = Get-LatestSourceWriteUtc
$binarioFechaUtc = (Get-Item $exePath).LastWriteTimeUtc
if ($fuenteReciente -and $fuenteReciente -gt $binarioFechaUtc) {
    Write-Warning (
        "El binario ($($binarioFechaUtc.ToLocalTime())) es mas antiguo que el codigo fuente " +
        "(cambio mas reciente: $($fuenteReciente.ToLocalTime())). Recompilando antes de arrancar..."
    )
    $raizDesktop = Join-Path $PSScriptRoot 'apps\desktop'
    Push-Location $raizDesktop
    try {
        # Directo con node al vite local, no via `pnpm exec`: el shim de pnpm 12.4.2 en este equipo
        # esta roto (falla con MODULE_NOT_FOUND buscando pnpm.cjs), y esto no depende de que
        # pnpm funcione, solo de que node_modules ya este instalado.
        & node (Join-Path $raizDesktop 'node_modules\vite\bin\vite.js') build
        if ($LASTEXITCODE -ne 0) {
            throw "vite build fallo (codigo de salida $LASTEXITCODE)."
        }
        & cargo build --locked --features custom-protocol --manifest-path src-tauri\Cargo.toml
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build fallo (codigo de salida $LASTEXITCODE)."
        }
    } finally {
        Pop-Location
    }
    Write-Output 'Recompilado.'
}

# El colector solo arranca si SensorAgent.exe coincide con el manifiesto firmado que lo acompana
# (ADR-0004). Si se recompila o se republica sin volver a firmar, la aplicacion arranca pero sin
# datos ("Collector unavailable") y sin decir por que. Se avisa aqui, con el comando exacto.
# Solo compara el hash con el manifiesto; la firma la verifica la propia aplicacion al arrancar.
function Test-Colector([string]$directorioExe) {
    $carpeta = @((Join-Path $directorioExe 'collector'), $directorioExe) |
        Where-Object { Test-Path (Join-Path $_ 'SensorAgent.exe') } |
        Select-Object -First 1
    if (-not $carpeta) { return 'no se encuentra SensorAgent.exe junto al ejecutable' }
    $manifiesto = Join-Path $carpeta 'release-manifest.json'
    if (-not (Test-Path $manifiesto)) { return "falta release-manifest.json en $carpeta" }
    $listado = (Get-Content $manifiesto -Raw | ConvertFrom-Json).files |
        Where-Object { $_.path -eq 'SensorAgent.exe' } |
        Select-Object -First 1
    if (-not $listado) { return 'el manifiesto no lista SensorAgent.exe' }
    $real = (Get-FileHash (Join-Path $carpeta 'SensorAgent.exe') -Algorithm SHA256).Hash
    if ($real -ne $listado.sha256) { return 'SensorAgent.exe no coincide con el manifiesto firmado' }
    return $null
}

$motivoColector = Test-Colector (Split-Path $exePath)
if ($motivoColector) {
    Write-Warning "El colector no va a arrancar: $motivoColector. La aplicacion se abrira, pero sin datos ('Collector unavailable')."
    Write-Warning 'Para repararlo, desde la raiz del repositorio:  node scripts/prepare-collector-bundle.mjs'
    Write-Warning 'y despues, en apps\desktop:  cargo build --locked --features custom-protocol --manifest-path src-tauri\Cargo.toml'
}

Write-Output "Arrancando ThrottleWatch ($exePath)..."
$proceso = Start-Process -FilePath $exePath -PassThru
Start-Sleep -Seconds 2

if (Get-Process -Id $proceso.Id -ErrorAction SilentlyContinue) {
    Write-Output "ThrottleWatch arrancada (PID $($proceso.Id))."
} else {
    Write-Error "ThrottleWatch se cerro justo despues de arrancar; revisa los registros en %LOCALAPPDATA%\ThrottleWatch\logs."
    exit 1
}
