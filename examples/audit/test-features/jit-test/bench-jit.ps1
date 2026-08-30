# bench-jit.ps1 - Benchmark de compilacion JIT (fib(20)) con timing por fase.
# Corrida 1 = cache miss (compila CLS->WASM + backend), Corrida 2 = cacheada.
# Uso: powershell -File bench-jit.ps1
#
# Migracion dev-2 (Fase 7): el bench original (bench5000.clsx con
# 1063 lineas de suma(2,3)) generaba 1200ms en Cranelift por la
# generacion de codigo nativo de 5000 llamadas, dando una impresion
# exagerada. Se reemplazo por bench-realistic.clsx (fib(20), 13 lineas)
# que es un caso de uso mas real: ~62ms Cranelift, ~10ms emit, ~1ms typecheck.
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
$clx = (Resolve-Path (Join-Path $root "target\debug\clx.exe")).Path
$bench = Join-Path $PSScriptRoot "bench-realistic.clsx"
$env:CLS_JIT_TIMING = "1"

Write-Host "== Benchmark compilacion JIT (bench-realistic.clsx, fib(20)) =="
Write-Host ""
Write-Host "--- Corrida 1 (cache miss) ---"
& $clx run --jit $bench 2>&1 | ForEach-Object { $_.ToString() } | Select-String -Pattern "\[JIT-TIMING\]" | ForEach-Object { $_.Line.Trim() }
Write-Host ""
Write-Host "--- Corrida 2 (cacheada) ---"
& $clx run --jit $bench 2>&1 | ForEach-Object { $_.ToString() } | Select-String -Pattern "\[JIT-TIMING\]" | ForEach-Object { $_.Line.Trim() }
Write-Host ""
Write-Host "--- Tiempo total del proceso ---"
$t1 = Measure-Command { & $clx run --jit $bench 2>&1 | Out-Null }
$t2 = Measure-Command { & $clx run --jit $bench 2>&1 | Out-Null }
Write-Host ("corrida miss (nuevo proceso): {0} ms" -f [math]::Round($t1.TotalMilliseconds))
Write-Host ("corrida cacheada (nuevo proceso): {0} ms" -f [math]::Round($t2.TotalMilliseconds))
Remove-Item Env:\CLS_JIT_TIMING
