<#
.SYNOPSIS
    Analiza el proyecto MiFutbolRS con todo el toolchain de Cargo.

.DESCRIPTION
    Ejecuta fetch, update (dry-run por defecto), check, build debug/release,
    pruebas (nextest si esta disponible), fmt, clippy, doc y, con -Completo,
    auditoria de seguridad (audit/deny), bloat y expand.

    Genera logs por paso y un reporte Markdown/JSON en target\analisis.

.PARAMETER Completo
    Incluye test --release, cargo audit, cargo deny, cargo bloat y cargo expand.

.PARAMETER Actualizar
    Ejecuta cargo update de verdad (sin el switch solo hace --dry-run).

.PARAMETER AplicarFix
    Ejecuta cargo fix --allow-dirty --allow-staged (modifica archivos).

.PARAMETER AbrirDoc
    Ejecuta cargo doc --open al final (abre el navegador).

.EXAMPLE
    .\scripts\analizar.ps1
    .\scripts\analizar.ps1 -Completo
    .\scripts\analizar.ps1 -Completo -Actualizar
#>
[CmdletBinding()]
param(
    [switch]$Completo,
    [switch]$Actualizar,
    [switch]$AplicarFix,
    [switch]$AbrirDoc,
    [string]$Salida = "target\analisis"
)

$ErrorActionPreference = "Continue"
$raiz = Split-Path -Parent $PSScriptRoot
Set-Location $raiz

New-Item -ItemType Directory -Force -Path $Salida | Out-Null
$resultados = [System.Collections.Generic.List[object]]::new()
$fallos = 0
$avisos = 0
$inicioGlobal = Get-Date

function Get-Tool([string]$nombre) {
    Get-Command $nombre -ErrorAction SilentlyContinue
}

$nextest = Get-Tool "cargo-nextest"
$audit = Get-Tool "cargo-audit"
$deny = Get-Tool "cargo-deny"
$bloat = Get-Tool "cargo-bloat"
$expand = Get-Tool "cargo-expand"
$watch = Get-Tool "cargo-watch"

function Invoke-Paso {
    param(
        [Parameter(Mandatory = $true)][string]$Nombre,
        [Parameter(Mandatory = $true)][scriptblock]$Comando,
        [string]$Archivo = "",
        [switch]$Opcional
    )

    if ([string]::IsNullOrWhiteSpace($Archivo)) {
        $Archivo = ($Nombre.ToLower() -replace '[^a-z0-9]+', '-').Trim('-')
    }
    $log = Join-Path $Salida "$Archivo.log"
    Write-Host ""
    Write-Host "=== $Nombre ===" -ForegroundColor Cyan
    $textoComando = $Comando.ToString().Trim()
    Write-Host "> $textoComando" -ForegroundColor DarkGray

    $reloj = [System.Diagnostics.Stopwatch]::StartNew()
    & $Comando 2>&1 | Tee-Object -FilePath $log
    if (-not (Test-Path $log)) {
        New-Item -ItemType File -Path $log | Out-Null
    }
    $codigo = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
    $reloj.Stop()

    $estado = if ($codigo -eq 0) { "OK" } elseif ($Opcional) { "OPCIONAL CON HALLAZGOS" } else { "FALLO" }
    if ($codigo -ne 0) {
        if ($Opcional) { $script:avisos++ } else { $script:fallos++ }
    }

    $script:resultados.Add([pscustomobject]@{
        Paso    = $Nombre
        Comando = $textoComando
        Estado  = $estado
        Segundos = [math]::Round($reloj.Elapsed.TotalSeconds, 1)
        Log     = $log
    })

    Write-Host ("  -> {0} ({1:N1}s)" -f $estado, $reloj.Elapsed.TotalSeconds) -ForegroundColor $(if ($codigo -eq 0) { "Green" } elseif ($Opcional) { "Yellow" } else { "Red" })
}

Write-Host "MiFutbolRS - analisis completo" -ForegroundColor Magenta
Write-Host ("Raiz: {0}" -f $raiz)
Write-Host ("Salida de logs: {0}" -f (Join-Path $raiz $Salida))

Invoke-Paso -Nombre "Versiones" -Comando { cargo --version; rustc --version } -Archivo "entorno"
Invoke-Paso -Nombre "Fetch de dependencias" -Comando { cargo fetch } -Archivo "fetch"

if ($Actualizar) {
    Invoke-Paso -Nombre "Actualizar dependencias" -Comando { cargo update } -Archivo "update"
} else {
    Invoke-Paso -Nombre "Actualizar dependencias (dry-run)" -Comando { cargo update --dry-run } -Archivo "update-dry-run"
}

Invoke-Paso -Nombre "Check (todos los targets)" -Comando { cargo check --all-targets } -Archivo "check"
Invoke-Paso -Nombre "Build debug" -Comando { cargo build } -Archivo "build"
Invoke-Paso -Nombre "Build release" -Comando { cargo build --release } -Archivo "build-release"

if ($nextest) {
    Invoke-Paso -Nombre "Pruebas (nextest)" -Comando { cargo nextest run --no-fail-fast } -Archivo "test-nextest"
    Invoke-Paso -Nombre "Doc-tests" -Comando { cargo test --doc } -Archivo "test-doc"
} else {
    Invoke-Paso -Nombre "Pruebas (cargo test)" -Comando { cargo test --all-targets } -Archivo "test"
}

Invoke-Paso -Nombre "Formato" -Comando { cargo fmt --check } -Archivo "fmt-check"
Invoke-Paso -Nombre "Clippy (warnings = errores)" -Comando { cargo clippy --all-targets -- -D warnings } -Archivo "clippy"
Invoke-Paso -Nombre "Documentacion" -Comando { cargo doc --no-deps } -Archivo "doc"

if ($AplicarFix) {
    Invoke-Paso -Nombre "Cargo fix" -Comando { cargo fix --allow-dirty --allow-staged --all-targets } -Archivo "fix"
}

if ($Completo) {
    if ($nextest) {
        Invoke-Paso -Nombre "Pruebas release (nextest)" -Comando { cargo nextest run --release --no-fail-fast } -Archivo "test-release"
    } else {
        Invoke-Paso -Nombre "Pruebas release" -Comando { cargo test --release } -Archivo "test-release"
    }

    if ($audit) {
        Invoke-Paso -Nombre "Auditoria de seguridad (cargo audit)" -Comando { cargo audit } -Archivo "audit" -Opcional
    } else {
        Write-Host "cargo-audit no esta instalado: se omite." -ForegroundColor Yellow
    }

    if ($deny) {
        Invoke-Paso -Nombre "Licencias y bans (cargo deny)" -Comando { cargo deny check } -Archivo "deny" -Opcional
    } else {
        Write-Host "cargo-deny no esta instalado: se omite." -ForegroundColor Yellow
    }

    if ($bloat) {
        Invoke-Paso -Nombre "Tamano del binario (cargo bloat)" -Comando { cargo bloat --release --crates -n 20 } -Archivo "bloat" -Opcional
    } else {
        Write-Host "cargo-bloat no esta instalado: se omite." -ForegroundColor Yellow
    }

    if ($expand) {
        Write-Host ""
        Write-Host "=== Expansion de macros (cargo expand --lib) ===" -ForegroundColor Cyan
        $expandArchivo = Join-Path $Salida "expand-lib.rs"
        $reloj = [System.Diagnostics.Stopwatch]::StartNew()
        cargo expand --lib 2>&1 | Out-File -FilePath $expandArchivo -Encoding utf8
        $codigo = $LASTEXITCODE
        $reloj.Stop()
        $estado = if ($codigo -eq 0) { "OK" } else { "OPCIONAL CON HALLAZGOS" }
        if ($codigo -ne 0) { $script:avisos++ }
        $script:resultados.Add([pscustomobject]@{
            Paso    = "Expansion de macros"
            Comando = "cargo expand --lib"
            Estado  = $estado
            Segundos = [math]::Round($reloj.Elapsed.TotalSeconds, 1)
            Log     = $expandArchivo
        })
        Write-Host ("  -> {0} ({1:N1}s)" -f $estado, $reloj.Elapsed.TotalSeconds) -ForegroundColor $(if ($codigo -eq 0) { "Green" } else { "Yellow" })
    } else {
        Write-Host "cargo-expand no esta instalado: se omite." -ForegroundColor Yellow
    }

    if ($bloat) {
        Write-Host ""
        Write-Host "=== Restaurando binario release (sin simbolos) ===" -ForegroundColor Cyan
        (Get-Item "src\main.rs").LastWriteTime = Get-Date
        Invoke-Paso -Nombre "Restaurar binario release" -Comando { cargo build --release } -Archivo "build-release-final"
    }
}

if ($AbrirDoc) {
    Write-Host ""
    Write-Host "Abriendo documentacion en el navegador..." -ForegroundColor Cyan
    cargo doc --no-deps --open
}

# Reporte
$duracion = (Get-Date) - $inicioGlobal
$reporteMd = Join-Path $Salida "reporte.md"
$reporteJson = Join-Path $Salida "reporte.json"

$lineas = [System.Collections.Generic.List[string]]::new()
$lineas.Add("# Reporte de analisis - MiFutbolRS")
$lineas.Add("")
$lineas.Add("- Fecha: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')")
$lineas.Add("- Raiz: $raiz")
$lineas.Add("- Duracion total: $([math]::Round($duracion.TotalSeconds, 1)) s")
$lineas.Add("- Entorno: $((cargo --version) -join ' ') / $((rustc --version) -join ' ')")
$lineas.Add("")
$lineas.Add("| Paso | Comando | Estado | Tiempo (s) | Log |")
$lineas.Add("|------|---------|--------|-----------:|-----|")
foreach ($r in $resultados) {
    $logRel = $r.Log.Replace($raiz + [IO.Path]::DirectorySeparatorChar, "")
    $lineas.Add("| $($r.Paso) | $($r.Comando) | $($r.Estado) | $($r.Segundos) | [$logRel]($logRel) |")
}
$lineaPruebas = $null
$logPruebas = Join-Path $Salida "test-nextest.log"
if (Test-Path $logPruebas) {
    $coincidencia = Select-String -Path $logPruebas -Pattern "Summary" | Select-Object -Last 1
    if ($coincidencia) { $lineaPruebas = $coincidencia.Line.Trim() }
}
if ($lineaPruebas) {
    $lineas.Add("")
    $lineas.Add("## Pruebas")
    $lineas.Add("")
    $lineas.Add("Resumen de nextest:")
    $lineas.Add("")
    $lineas.Add($lineaPruebas)
}

$lineas.Add("")
$lineas.Add("## Resultado")
$lineas.Add("")
$lineas.Add("- Pasos ejecutados: $($resultados.Count)")
$lineas.Add("- Fallos: $fallos")
$lineas.Add("- Avisos/hallazgos opcionales: $avisos")
$lineas.Add("")
$lineas.Add("## Herramientas opcionales")
$lineas.Add("")
foreach ($par in @(
    @("cargo-nextest", $nextest), @("cargo-audit", $audit), @("cargo-deny", $deny),
    @("cargo-watch", $watch), @("cargo-bloat", $bloat), @("cargo-expand", $expand)
)) {
    $estadoTool = if ($par[1]) { "instalado" } else { "no instalado" }
    $lineas.Add("- $($par[0]): $estadoTool")
}
$lineas.Add("")
$lineas.Add("> cargo run / cargo run --release / cargo watch y cargo clean no se ejecutan en el analisis: son interactivos o destructivos.")

$lineas | Set-Content -Path $reporteMd -Encoding utf8
$resultados | ConvertTo-Json -Depth 4 | Set-Content -Path $reporteJson -Encoding utf8

Write-Host ""
Write-Host "================ RESUMEN ================" -ForegroundColor Magenta
$resultados | Format-Table Paso, Estado, Segundos -AutoSize | Out-Host
Write-Host ("Reporte: {0}" -f $reporteMd) -ForegroundColor Cyan
Write-Host ("Fallos: {0} - Avisos: {1}" -f $fallos, $avisos) -ForegroundColor $(if ($fallos -eq 0) { "Green" } else { "Red" })

if ($fallos -gt 0) { exit 1 }
