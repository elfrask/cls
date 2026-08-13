# _release-b.ps1 - SECCION B: regresion completa (validacion, fase-1-r2, fase-2-r2, features)
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== B: REGRESION ====="

function Run-Suite($suiteLabel, $dir, $expectedPass = -1) {
    "----- SUITE: $suiteLabel -----"
    $pass = 0; $fail = 0
    $failList = @()
    Get-ChildItem -Path $dir -Filter *.clsx | Sort-Object Name | ForEach-Object {
        $label = "$suiteLabel-$($_.BaseName)"
        $code = Run-Jit $label $_.FullName
        $bad = Check-BadStrings $label
        if ($code -eq 0 -and -not $bad) { $pass++ } else { $fail++; $failList += "$($_.BaseName) exit=$code bad=$bad" }
    }
    "PASS=$pass FAIL=$fail (esperado: $expectedPass)"
    if ($failList.Count -gt 0) { $failList | ForEach-Object { "  FAILED: $_" } }
    if ($fail -eq 0) { Add-Result $suiteLabel "PASS($pass)" } else { Add-Result $suiteLabel "FAIL($pass/$fail)" }
}

# 4. validacion (scripts de cierre)
"----- SUITE: validacion -----"
$pass = 0; $fail = 0
Get-ChildItem -Path (Join-Path $audit "validacion") -Filter *.clsx | Sort-Object Name | ForEach-Object {
    $label = "val-$($_.BaseName)"
    $code = Run-Jit $label $_.FullName
    $bad = Check-BadStrings $label
    if ($code -eq 0 -and -not $bad) { $pass++ } else { $fail++; "  FAILED: $($_.BaseName) exit=$code bad=$bad" }
}
"PASS=$pass FAIL=$fail"
if ($fail -eq 0) { Add-Result "validacion" "PASS($pass)" } else { Add-Result "validacion" "FAIL($pass/$fail)" }

# _runner.ps1 y B-floats.ps1 de validacion (comparadores originales)
"----- validacion/_runner.ps1 (v-pow-var) -----"
& (Join-Path $audit "validacion\_runner.ps1") -Name "release-v-pow-var" -Args "run --jit C:\Users\Frask\Documents\cls\examples\audit\validacion\v-pow-var.clsx" | Out-Null
"exit=$LASTEXITCODE"

"----- validacion/B-floats.ps1 (fase-1-r2) -----"
$bf = & (Join-Path $audit "validacion\B-floats.ps1") 2>&1
$bf

# 5. fase-1-r2
Run-Suite "f1" (Join-Path $audit "fase-1-r2") 57

# 6. fase-2-r2
Run-Suite "f2" (Join-Path $audit "fase-2-r2") 20

# 7. features
Run-Suite "feat" (Join-Path $audit "features") 18

"===== RESUMEN B ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
