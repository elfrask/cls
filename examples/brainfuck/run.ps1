# run.ps1 - Orquestador de la prueba Brainfuck de CLS.
#
#   1. compila el oraculo C (bf.dll + bf_oracle.exe) si falta o esta viejo
#   2. (re)genera los .out dorados con el oraculo
#   3. corre el runner CLS (que compara CLS puro vs C vs dorado)
#
# Uso:  powershell -File examples\brainfuck\run.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH y target\debug\clx.exe compilado.

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$oracleDir = Join-Path $here "oracle"
$progsDir = Join-Path $here "programs"
$src = Join-Path $here "oracle\bf.c"
$clx = Join-Path $repo "target\debug\clx.exe"

# --- herramienta de compilacion ---
$gcc = Get-Command gcc -ErrorAction SilentlyContinue
if (-not $gcc) {
    Write-Host "ERROR: no se encontro 'gcc' (MinGW-w64 x64) en PATH."
    Write-Host "Compila el oraculo a mano:"
    Write-Host "  gcc -shared -O2 -o oracle\bf.dll oracle\bf.c"
    Write-Host "  gcc -O2 -DBF_CLI -o oracle\bf_oracle.exe oracle\bf.c"
    exit 2
}

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

# --- 0. modulos compartidos de /modules que consume la prueba ---
#   io    : lectura de archivos (readText/readBytes) via FFI
#   (bytes quedo CLS puro y sin build: usa los intrinsecos char/ord)
foreach ($mod in @("io")) {
    $b = Join-Path $repo "modules\$mod\build.ps1"
    if (-not (Test-Path $b)) { Write-Host "ERROR: no existe $b"; exit 2 }
    & $b
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: modules/$mod/build.ps1 fallo."; exit 1 }
}

# --- 1. compilar oraculo si falta o esta viejo ---
$dll = Join-Path $oracleDir "bf.dll"
$cli = Join-Path $oracleDir "bf_oracle.exe"
$needBuild = (-not (Test-Path $dll)) -or (-not (Test-Path $cli)) `
    -or ((Get-Item $src).LastWriteTime -gt (Get-Item $dll).LastWriteTime)
if ($needBuild) {
    Write-Host "== compilando oraculo C (bf.dll + bf_oracle.exe) =="
    & gcc -shared -O2 -o $dll $src
    if ($LASTEXITCODE -ne 0) { exit 1 }
    & gcc -O2 -DBF_CLI -o $cli $src
    if ($LASTEXITCODE -ne 0) { exit 1 }
} else {
    Write-Host "== oraculo C ya compilado =="
}

# --- 2. regenerar los .out dorados con el oraculo ---
Write-Host "== regenerando salidas doradas (.out) =="
$programs = @(
    @{ name = "hello";   input = $null },
    @{ name = "wrap";    input = $null },
    @{ name = "torture"; input = $null },
    @{ name = "bench";   input = $null },
    @{ name = "ptr";     input = $null },
    @{ name = "cat";     input = (Join-Path $progsDir "cat.in") }
)
foreach ($p in $programs) {
    $bf = Join-Path $progsDir ($p.name + ".bf")
    $out = Join-Path $progsDir ($p.name + ".out")
    if ($null -ne $p.input) {
        & $cli $bf --in $p.input --out $out 2>$null
    } else {
        & $cli $bf --out $out 2>$null
    }
    if ($LASTEXITCODE -ne 0) { Write-Host "FALLO oraculo en $($p.name)"; exit 1 }
}

# --- 3. correr el runner CLS desde la raiz del repo ---
Write-Host ""
Push-Location $repo
try {
    & $clx run "examples\brainfuck\src\main.clsx" --add-path-module=modules
    $code = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $code
