# _release-a2.ps1 - SECCION A (segunda pasada): variantes X2/X3
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== A2: VARIANTES ====="

$cases = @(
    @{ n = "x2-validos";            e = 0 },
    @{ n = "x2-int-float";          e = 1 },
    @{ n = "x3-var-noalias";        e = 0 },
    @{ n = "x3-var-mathnoalias";    e = 0 }
)

foreach ($c in $cases) {
    $file = Join-Path $audit "release\$($c.n).clsx"
    $code = Run-Jit $c.n $file
    $bad = Check-BadStrings $c.n
    $status = if ($code -eq $c.e -and -not $bad) { "PASS" } else { "FAIL" }
    Add-Result $c.n $status
    "--- $($c.n) exit=$code (esperado $($c.e)) bad=$bad ---"
}

# x2-int-float: verificar que el error es el mensaje esperado
$errFile = Join-Path $logDir "x2-int-float.err.txt"
$errTxt = Get-Content $errFile -Raw
"x2-int-float error menciona Float/Int: " + ($errTxt -match "Float no asignable a Int")
"x2-int-float tiene caret: " + ($errTxt -match "\^")

$script:results | Format-Table -AutoSize | Out-String | Write-Output
