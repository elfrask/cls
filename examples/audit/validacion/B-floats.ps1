$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logDir = "C:\Users\Frask\Documents\cls\examples\audit\validacion\_logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$script:lastCode = 0
function Run-Jit($label, $path) {
    $out = & $clx run --jit $path 2>&1
    $script:lastCode = $LASTEXITCODE
    $first = ($out | Select-Object -First 6) -join " | "
    Write-Output ("{0}`tEXIT={1}`t{2}" -f $label, $script:lastCode, $first)
}

"===== B: FASE-1-R2 (c2-*, r1-*, r2-*, r3-*, r4-*) ====="
$dir = "C:\Users\Frask\Documents\cls\examples\audit\fase-1-r2"
$results = @{}
$pass = 0; $fail = 0
Get-ChildItem -Path $dir -Filter *.clsx | Sort-Object Name | ForEach-Object {
    Run-Jit $_.BaseName $_.FullName
    $code = $script:lastCode
    $results[$_.BaseName] = $code
    if ($code -eq 0) { $pass++ } else { $fail++ }
}
""
"PASS=$pass FAIL=$fail"
"----- FAILED (exit != 0) -----"
$results.GetEnumerator() | Where-Object { $_.Value -ne 0 } | Sort-Object Name | ForEach-Object { $_.Key }
