# _release-a.ps1 - SECCION A: criticos X1/X2/X3 (bateria de limites)
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== A: CRITICOS ====="

$scripts = @(
    "x1-pow.clsx",
    "x2-compound.clsx",
    "x3-collision.clsx",
    "x3-same-alias.clsx",
    "x3-user-json.clsx",
    "x3-user-math.clsx"
)

foreach ($s in $scripts) {
    $file = Join-Path $audit "release\$s"
    $code = Run-Jit $s $file
    $bad = Check-BadStrings $s
    $status = if ($code -eq 0 -and -not $bad) { "PASS" } elseif ($bad) { "BADSTRING:$bad" } else { "FAIL(exit=$code)" }
    Add-Result $s $status
    "--- $s ---"
    "exit=$code badstring=$bad"
}

"===== RESUMEN A ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
