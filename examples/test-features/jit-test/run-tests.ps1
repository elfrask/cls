# run-tests.ps1 — Prueba cada feature del JIT comparando salidas JIT vs walker (paridad).
# Uso: powershell -File run-tests.ps1
$ErrorActionPreference = "SilentlyContinue"
$clx = Join-Path $PSScriptRoot "..\..\..\target\debug\clx.exe"
$units = Join-Path $PSScriptRoot "units"
$clx = (Resolve-Path $clx).Path

# Scripts que requieren condiciones especiales (no son fallos).
$special = @{
    "a11.clsx"     = "requiere DLL de extension (clx_demo.dll)"
    "f64arr.clsx"  = "requiere args de app (-- ola mundo)"
    "b7.clsx"      = "requiere red (httpbin.org)"
    "synerr.clsx"  = "prueba de error de sintaxis (sin paridad)"
    "syn2.clsx"    = "prueba de error de sintaxis (sin paridad)"
    "a2min.clsx"   = "variante de tuplas (cubierto por a2)"
    "bench_fib.clsx" = "benchmark (usa now(), no hay paridad de tiempos)"
}

$pass = 0; $fail = 0; $skip = 0
Write-Host "== Test de features JIT (paridad JIT == walker) =="
Write-Host ("{0,-14} {1,-6} {2}" -f "SCRIPT", "RES", "NOTA")
Write-Host ("{0,-14} {1,-6} {2}" -f "------", "---", "----")
foreach ($f in Get-ChildItem $units -Filter *.clsx | Sort-Object Name) {
    $name = $f.Name
    if ($special.ContainsKey($name)) {
        Write-Host ("{0,-14} {1,-6} {2}" -f $name, "SKIP", $special[$name])
        $skip++
        continue
    }
    $jit = (& $clx run --jit $f.FullName 2>$null | Out-String)
    $walk = (& $clx run $f.FullName 2>$null | Out-String)
    if ($jit -eq $walk) {
        Write-Host ("{0,-14} {1,-6} {2}" -f $name, "PASS", "")
        $pass++
    } else {
        Write-Host ("{0,-14} {1,-6} {2}" -f $name, "FAIL", "")
        $fail++
    }
}
Write-Host ""
Write-Host "Resultado: $pass PASS, $fail FAIL, $skip SKIP"
