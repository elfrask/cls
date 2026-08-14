# run-bench.ps1 — Benchmarks CLS con múltiples módulos (walker vs JIT).
# El proyecto está en examples/benchmarks-modules (src/main.clsx + lib/).
# Uso: powershell -File run-bench.ps1
$ErrorActionPreference = "SilentlyContinue"
$clx = Join-Path $PSScriptRoot "..\..\target\debug\clx.exe"
$clx = (Resolve-Path $clx).Path
$main = Join-Path $PSScriptRoot "src\main.clsx"

Write-Host "=============================================="
Write-Host "  Benchmarks CLS (múltiples módulos)"
Write-Host "=============================================="
Write-Host ""
Write-Host "== TREE-WALKER DEPRECADO (clx run --ast-walker) =="
$t0 = Get-Date
& $clx run --ast-walker $main 2>&1 | ForEach-Object { Write-Host $_ }
$tw = ((Get-Date) - $t0).TotalMilliseconds
Write-Host ("Walker total: {0:N0} ms" -f $tw)
Write-Host ""
Write-Host "== JIT/WASM (clx run --jit) =="
Write-Host "(El JIT multi-módulo tiene una limitación conocida: los Span del type map"
Write-Host " no incluyen el archivo, por lo que módulos y main colisionan en coordenadas.)"
$t0 = Get-Date
& $clx run --jit $main 2>&1 | ForEach-Object { Write-Host $_ }
$jit = ((Get-Date) - $t0).TotalMilliseconds
Write-Host ("JIT total: {0:N0} ms" -f $jit)
Write-Host ""
Write-Host "Speedup JIT/walker: ver README"
