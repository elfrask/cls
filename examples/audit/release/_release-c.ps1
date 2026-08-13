# _release-c.ps1 - SECCION C: errores estandar + stress + perf
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== C1: ERRORES ESTANDAR (esperan exit != 0 y trace completo) ====="

$errCases = @(
    @{ n = "err-divzero-nested";      f = "errores\err-divzero-nested.clsx" },
    @{ n = "err-error-en-modulo";     f = "modules\err-error-en-modulo.clsx" },
    @{ n = "err-throw-no-capturado";  f = "errores\err-throw-no-capturado.clsx" },
    @{ n = "err-index-fuera";         f = "errores\err-index-fuera.clsx" },
    @{ n = "err-miembro-inexistente"; f = "errores\err-miembro-inexistente.clsx" },
    @{ n = "sint-break-fuera";        f = "cmx\sint-break-fuera.clsx" },
    @{ n = "err-conversion";          f = "errores\err-conversion.clsx" }
)

foreach ($c in $errCases) {
    $file = Join-Path $audit $c.f
    $code = Run-Jit $c.n $file
    $bad = Check-BadStrings $c.n
    $outFile = Join-Path $logDir "$($c.n).out.txt"
    $errFile = Join-Path $logDir "$($c.n).err.txt"
    $errTxt = (Get-Content $errFile -Raw -ErrorAction SilentlyContinue)
    $hasError = ($errTxt -match "Error") -or ($errTxt -match "error")
    $status = if ($code -ne 0 -and $hasError -and -not $bad) { "PASS" } else { "FAIL" }
    Add-Result $c.n $status
    "--- $($c.n) exit=$code bad=$bad hasErrorMsg=$hasError ---"
    if ($errTxt) { $errTxt | ForEach-Object { $_ } }
}

"===== C2: STRESS ====="

$stressCases = @(
    @{ n = "stress-aritmetica";     f = "stress\stress-aritmetica.clsx";        t = 60000 },
    @{ n = "stress-prints";         f = "stress\stress-prints.clsx";            t = 60000 },
    @{ n = "stress-array-1m";       f = "stress\stress-array-1m.clsx";          t = 60000 },
    @{ n = "stress-recursion";      f = "stress\stress-recursion.clsx";         t = 60000 },
    @{ n = "stress-fact-100k";      f = "stress\stress-fact-100k.clsx";         t = 60000 },
    @{ n = "stress-infinite-while"; f = "stress\stress-infinite-while.clsx";    t = 15000 }
)

foreach ($c in $stressCases) {
    $file = Join-Path $audit $c.f
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $code = Run-Jit $c.n $file $c.t
    $sw.Stop()
    $bad = Check-BadStrings $c.n
    "--- $($c.n) exit=$code tiempo=$($sw.ElapsedMilliseconds)ms bad=$bad ---"
    if ($c.n -eq "stress-fact-100k" -or $c.n -eq "stress-recursion") {
        $errFile = Join-Path $logDir "$($c.n).err.txt"
        $errTxt = (Get-Content $errFile -Raw -ErrorAction SilentlyContinue)
        $clean = ($errTxt -match "stack overflow") -or ($errTxt -match "stack")
        $status = if ($code -ne 0 -and $clean -and -not $bad) { "PASS" } else { "FAIL" }
        Add-Result $c.n $status
        "  stack-msg-clean=$clean"
    } elseif ($c.n -eq "stress-infinite-while") {
        $status = if ($code -eq "TIMEOUT") { "PASS(expected)" } else { "FAIL" }
        Add-Result $c.n $status
    } else {
        $status = if ($code -eq 0 -and -not $bad) { "PASS" } else { "FAIL" }
        Add-Result $c.n $status
    }
}

"===== C3: PERF ====="

$perfCases = @(
    @{ n = "perf-loop";    f = "perf\perf-loop.clsx" },
    @{ n = "perf-fib";     f = "perf\perf-fib.clsx" },
    @{ n = "perf-llamadas";f = "perf\perf-llamadas.clsx" },
    @{ n = "perf-math";    f = "perf\perf-math.clsx" }
)

foreach ($c in $perfCases) {
    $file = Join-Path $audit $c.f
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $code = Run-Jit $c.n $file 60000
    $sw.Stop()
    $bad = Check-BadStrings $c.n
    $outFile = Join-Path $logDir "$($c.n).out.txt"
    $outTxt = (Get-Content $outFile -Raw -ErrorAction SilentlyContinue)
    $status = if ($code -eq 0 -and -not $bad) { "PASS" } else { "FAIL" }
    Add-Result $c.n $status
    "--- $($c.n) exit=$code walltime=$($sw.ElapsedMilliseconds)ms bad=$bad ---"
    if ($outTxt) { $outTxt }
}

"===== RESUMEN C ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
