# run.ps1 - Orquestador de la suite de conformidad de CLS 2.0.
#
#   1. compila el oraculo C (oracle.dll) si falta o esta viejo
#   2. corre la suite (src/main.clsx) en cache FRIA y CALIENTE y compara salidas
#      (equivalente al oraculo 3 O0/O2, que no existe en CLS: detecta bugs de cache)
#   3. corre el fuzzer determinista (src/fuzz.clsx)
#   4. corre los repros de findings/ y reporta cuales siguen presentes
#   5. imprime un resumen; exit != 0 si CUALQUIER caso diverge
#
# Uso:  powershell -File examples\conformance\run.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH y target\debug\clx.exe compilado.

# NativeCommandError de stderr no debe abortar el script (wasmi/warnings van a stderr).
$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$oracleDir = Join-Path $here "oracle"
$src = Join-Path $oracleDir "oracle.c"
$dll = Join-Path $oracleDir "oracle.dll"
$clx = Join-Path $repo "target\debug\clx.exe"

# Corre clx capturando stdout+stderr en un solo string (sin que stderr aborte).
function RunClx {
    param([string[]]$ClxArgs)
    $o = [System.IO.Path]::GetTempFileName()
    $e = [System.IO.Path]::GetTempFileName()
    & $clx @ClxArgs 1> $o 2> $e
    $code = $LASTEXITCODE
    $text = ((Get-Content -Raw $o) + (Get-Content -Raw $e))
    Remove-Item $o, $e -ErrorAction SilentlyContinue
    return [pscustomobject]@{ Text = $text; Code = $code }
}

$gcc = Get-Command gcc -ErrorAction SilentlyContinue
if (-not $gcc) {
    Write-Host "ERROR: no se encontro 'gcc' (MinGW-w64 x64) en PATH."
    Write-Host "Compila el oraculo a mano: gcc -shared -O2 -o oracle\oracle.dll oracle\oracle.c"
    exit 2
}
if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

# --- 1. compilar oraculo si falta o esta viejo ---
$needBuild = (-not (Test-Path $dll)) -or ((Get-Item $src).LastWriteTime -gt (Get-Item $dll).LastWriteTime)
if ($needBuild) {
    Write-Host "== compilando oraculo C (oracle.dll) =="
    & gcc -shared -O2 -o $dll $src
    if ($LASTEXITCODE -ne 0) { exit 1 }
} else {
    Write-Host "== oraculo C ya compilado =="
}

$suiteFails = 0

Push-Location $repo
try {
    $main = "examples\conformance\src\main.clsx"
    $fuzz = "examples\conformance\src\fuzz.clsx"

    # --- 2. suite: cache fria vs caliente ---
    Write-Host ""
    Write-Host "== suite de conformidad (fria vs caliente) =="
    & $clx clean | Out-Null
    $cold = RunClx @("run", $main, "--add-path-module=modules")
    $warm = RunClx @("run", $main, "--add-path-module=modules")

    Write-Host $warm.Text

    if ($cold.Text.TrimEnd() -ne $warm.Text.TrimEnd()) {
        Write-Host "[FAIL] cache: la salida CAMBIA entre cache fria y caliente (bug de cache)"
        Write-Host "--- fria ---"; Write-Host $cold.Text
        Write-Host "--- caliente ---"; Write-Host $warm.Text
        $suiteFails = $suiteFails + 1
    } else {
        Write-Host "[ OK ] cache: salida identica en fria y caliente"
    }
    if ($warm.Code -ne 0) {
        Write-Host "[FAIL] suite: exit=$($warm.Code) (hay divergencias CLS vs C)"
        $suiteFails = $suiteFails + 1
    } else {
        Write-Host "[ OK ] suite: exit=0 (todos los ejes coinciden con C)"
    }

    # --- 3. fuzzer ---
    Write-Host ""
    Write-Host "== fuzzer determinista =="
    $fz = RunClx @("run", $fuzz, "--add-path-module=modules")
    Write-Host $fz.Text
    if ($fz.Code -ne 0) {
        Write-Host "[FAIL] fuzzer: exit=$($fz.Code)"
        $suiteFails = $suiteFails + 1
    } else {
        Write-Host "[ OK ] fuzzer: exit=0"
    }

    # --- 4. findings (cada uno reporta si el bug sigue presente) ---
    Write-Host ""
    Write-Host "== repros de findings (exit!=0 = bug PRESENTE) =="
    $findings = @(
        "01-finally-control-flow.clsx",
        "02-ffi-f64-args.clsx",
        "03-cstring-return-utf8.clsx",
        "04-for-each-closure-capture.clsx",
        "05-const-int-literal-arith.clsx",
        "06-closure-return-bool.clsx"
    )
    foreach ($f in $findings) {
        $path = "examples\conformance\findings\$f"
        $r = RunClx @("run", $path)
        $state = if ($r.Code -ne 0) { "BUG PRESENTE" } else { "resuelto" }
        Write-Host ("  [{0}] {1}" -f $state, $f)
        Write-Host $r.Text.TrimEnd()
    }

    # --- 5. wasmi (informacional; puede fallar por bugs preexistentes) ---
    Write-Host ""
    Write-Host "== wasmi (informacional; CLS_JIT_RUNTIME=wasmi) =="
    $env:CLS_JIT_RUNTIME = "wasmi"
    $wasmi = RunClx @("run", $main, "--add-path-module=modules")
    Remove-Item Env:\CLS_JIT_RUNTIME -ErrorAction SilentlyContinue
    Write-Host ("  exit={0}" -f $wasmi.Code)
    if ($wasmi.Code -ne 0) {
        Write-Host "  (wasmi puede fallar por soporte limitado de excepciones/FFI; NO cuenta como fallo de la suite)"
        Write-Host ("  " + (($wasmi.Text -split "`n" | Select-Object -First 3) -join " / "))
    }
}
finally {
    Pop-Location
}

Write-Host ""
if ($suiteFails -eq 0) {
    Write-Host "RESULTADO GLOBAL: OK"
    exit 0
}
Write-Host "RESULTADO GLOBAL: $suiteFails fallo(s) (ver findings/)"
exit 1
