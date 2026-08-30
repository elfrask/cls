# run-bench.ps1 - Benchmarks CLS con múltiples módulos (JIT).
# El proyecto está en examples/benchmarks-modules (src/main.clsx + lib/).
#
# Migracion dev-2 (Fase 7): el walker fue eliminado del repo. Este
# script ya no puede comparar walker vs JIT. Solo mide JIT.
#
# Uso: powershell -File run-bench.ps1
$ErrorActionPreference = "SilentlyContinue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$clx = (Resolve-Path (Join-Path $root "target\debug\clx.exe")).Path
$main = Join-Path $PSScriptRoot "src\main.clsx"

Write-Host "=============================================="
Write-Host "  Benchmarks CLS (múltiples módulos, JIT)"
Write-Host "=============================================="
Write-Host ""
Write-Host "== JIT/WASM (clx run) =="
Write-Host "(El JIT multi-módulo tiene una limitación conocida: los Span del type map"
Write-Host " no incluyen el archivo, por lo que módulos y main colisionan en coordenadas.)"
$t0 = Get-Date
& $clx run $main 2>&1 | ForEach-Object { Write-Host $_ }
$jit = ((Get-Date) - $t0).TotalMilliseconds
Write-Host ("JIT total: {0:N0} ms" -f $jit)
Write-Host ""
Write-Host "NOTA: La comparacion walker vs JIT ya no aplica. Walker eliminado en Fase 7."
