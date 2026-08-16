# bench-jit.ps1 - Benchmark de compilacion JIT (5000 lineas de llamadas a suma) con timing por fase.
# Corrida 1 = cache miss (compila CLS->WASM + backend), Corrida 2 = cacheada.
# Uso: powershell -File bench-jit.ps1
$root = Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$clx = Join-Path $root "target\debug\clx.exe"
$bench = Join-Path $PSScriptRoot "bench5000.clsx"
$env:CLS_JIT_TIMING = "1"

Write-Host "== Benchmark compilacion JIT (bench5000.clsx, 5000 llamadas) =="
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
