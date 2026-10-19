# replay.ps1 - Prueba #9 (snake): corre cada tests/*.replay y verifica el estado
# final contra (a) el `.final` versionado y (b) el oraculo Python (tests/oracle.py).
#
# Uso:  powershell -File examples\snake\replay.ps1
# Requiere: target\debug\clx.exe (Python opcional para el cross-check).

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}
function Norm([string]$t) { return ($t -replace "`r`n", "`n").Trim() }

Push-Location $here
$pass = 0
$fail = 0
try {
    $replays = @(Get-ChildItem "tests" -Filter *.replay | Sort-Object Name)
    foreach ($r in $replays) {
        $name = [System.IO.Path]::GetFileNameWithoutExtension($r.Name)
        $final = Join-Path $here ("tests\" + $name + ".final")
        $got = (& $clx run "src\main.clsx" "--add-path-module=../../modules" "--" $r.FullName 2> $null | Out-String)
        if ($LASTEXITCODE -ne 0) {
            Write-Host ("  [FAIL] {0}: exit={1}" -f $name, $LASTEXITCODE)
            $fail++
            continue
        }
        $g = Norm $got
        if (-not (Test-Path $final)) { Write-Host ("  [FAIL] {0}: falta {1}" -f $name, $final); $fail++; continue }
        $e = Norm (Get-Content -Raw -Encoding UTF8 $final)
        if ($g -ne $e) {
            Write-Host ("  [FAIL] {0}: diverge del .final" -f $name)
            Write-Host ("    cls: {0}" -f $g)
            Write-Host ("    fin: {0}" -f $e)
            $fail++
            continue
        }
        if (Get-Command python -ErrorAction SilentlyContinue) {
            $py = Norm (& python "tests\oracle.py" $r.FullName | Out-String)
            if ($py -ne $g) {
                Write-Host ("  [FAIL] {0}: oraculo diverge" -f $name)
                Write-Host ("    cls: {0}" -f $g)
                Write-Host ("    py : {0}" -f $py)
                $fail++
                continue
            }
        }
        Write-Host ("  [ OK ] {0}" -f $name)
        $pass++
    }
} finally {
    Pop-Location
}

Write-Host ""
if ($fail -eq 0) {
    Write-Host ("RESULTADO: OK ({0} replays)" -f $pass)
    exit 0
}
Write-Host ("RESULTADO: FAIL ({0}/{1})" -f $fail, ($pass + $fail))
exit 1
