# _release-d-cache.ps1 - SECCION D: cache CLS->WASM (MISS al editar, HIT sin editar, sin sobre-invalidacion)
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== D: CACHE ====="

$proj = Join-Path $audit "release\_cachetest"
Remove-Item $proj -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item (Join-Path $audit "modules\cachetest") $proj -Recurse
$main = Join-Path $proj "main.clsx"
$libmod = Join-Path $proj "libmod.clsx"
$env:CLS_JIT_TIMING = "1"

# D1: primera ejecucion -> MISS (clave nueva)
$code = Run-Jit "cache-1" $main
$err1 = Get-Content (Join-Path $logDir "cache-1.err.txt") -Raw
$miss1 = $err1 -match "miss"
$out1 = Get-Content (Join-Path $logDir "cache-1.out.txt") -Raw
"D1 primera run: exit=$code MISS=$miss1 out=$out1"
Add-Result "cache-1-miss" $(if ($miss1) { "PASS" } else { "FAIL" })

# D2: segunda run sin editar -> HIT
$code = Run-Jit "cache-2" $main
$err2 = Get-Content (Join-Path $logDir "cache-2.err.txt") -Raw
$hit2 = $err2 -match "HIT"
$out2 = Get-Content (Join-Path $logDir "cache-2.out.txt") -Raw
"D2 segunda run: exit=$code HIT=$hit2 out=$out2"
Add-Result "cache-2-hit" $(if ($hit2) { "PASS" } else { "FAIL" })

# D3: editar libmod (3 -> 4) -> MISS y el valor cambia
(Get-Content $libmod -Raw).Replace("return 3;", "return 4;") | Set-Content $libmod -NoNewline -Encoding ASCII
$code = Run-Jit "cache-3" $main
$err3 = Get-Content (Join-Path $logDir "cache-3.err.txt") -Raw
$miss3 = $err3 -match "miss"
$out3 = Get-Content (Join-Path $logDir "cache-3.out.txt") -Raw
$fresh3 = $out3 -match "VALOR_MODULO: 4"
"D3 edit libmod: exit=$code MISS=$miss3 valor-nuevo=$fresh3 out=$out3"
Add-Result "cache-3-invalidate" $(if ($miss3 -and $fresh3) { "PASS" } else { "FAIL" })

# D4: run de nuevo -> HIT
$code = Run-Jit "cache-4" $main
$err4 = Get-Content (Join-Path $logDir "cache-4.err.txt") -Raw
$hit4 = $err4 -match "HIT"
"D4 post-edit: exit=$code HIT=$hit4"
Add-Result "cache-4-hit" $(if ($hit4) { "PASS" } else { "FAIL" })

# D5: archivo NO relacionado en el mismo dir -> NO invalida (sigue HIT)
$unrelated = Join-Path $proj "no-relacionado.clsx"
Set-Content $unrelated "function main(args: String[]) -> int { return 0; };"
$code = Run-Jit "cache-5" $main
$err5 = Get-Content (Join-Path $logDir "cache-5.err.txt") -Raw
$hit5 = $err5 -match "HIT"
"D5 archivo ajeno: exit=$code HIT=$hit5"
Add-Result "cache-5-no-over-invalid" $(if ($hit5) { "PASS" } else { "FAIL" })

Remove-Item $env:USERPROFILE\.cache\cls\*.wasm -Force -ErrorAction SilentlyContinue
"===== RESUMEN D ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
