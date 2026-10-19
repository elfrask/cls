# diff.ps1 - Prueba #10 (scheme): corre cada tests/*.scm y compara con tests/*.out.
#
#   1. intérprete Scheme en CLS puro  vs  salida esperada (.out, versionada)
#   2. (opcional) cross-check con `guile` si está en el PATH
#
# Uso:  powershell -File examples\scheme\diff.ps1
# Requiere: target\debug\clx.exe

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

function Norm([string]$t) { return (($t -replace "`r`n", "`n").TrimEnd()) }

Push-Location $here
$pass = 0
$fail = 0
try {
    Write-Host "== 1. intérprete CLS vs tests/*.out =="
    $tests = @(Get-ChildItem "tests" -Filter *.scm | Sort-Object Name)
    foreach ($t in $tests) {
        $name = [System.IO.Path]::GetFileNameWithoutExtension($t.Name)
        $exp = Join-Path $here ("tests\" + $name + ".out")
        $got = (& $clx run "src\main.clsx" "--add-path-module=../../modules" "--" $t.FullName 2> $null | Out-String)
        if ($LASTEXITCODE -ne 0) {
            Write-Host ("  [FAIL] {0}: exit={1}" -f $name, $LASTEXITCODE)
            $fail++
            continue
        }
        if (-not (Test-Path $exp)) {
            Write-Host ("  [FAIL] {0}: falta {1}" -f $name, $exp)
            $fail++
            continue
        }
        $g = Norm $got
        $e = Norm (Get-Content -Raw -Encoding UTF8 $exp)
        if ($g -eq $e) {
            Write-Host ("  [ OK ] {0}" -f $name)
            $pass++
        } else {
            Write-Host ("  [FAIL] {0}" -f $name)
            Write-Host ("    cls : {0}" -f ($g -replace "`n", " / "))
            Write-Host ("    esp : {0}" -f ($e -replace "`n", " / "))
            $fail++
        }
    }

    Write-Host ""
    Write-Host "== 2. cross-check guile (opcional) =="
    if (Get-Command guile -ErrorAction SilentlyContinue) {
        $gfail = 0
        foreach ($t in $tests) {
            $name = [System.IO.Path]::GetFileNameWithoutExtension($t.Name)
            $exp = Join-Path $here ("tests\" + $name + ".out")
            $gout = (& guile --no-auto-compile -s $t.FullName 2> $null | Out-String)
            if ((Norm $gout) -ne (Norm (Get-Content -Raw -Encoding UTF8 $exp))) {
                Write-Host ("  [FAIL] guile {0}" -f $name)
                $gfail++
            }
        }
        if ($gfail -eq 0) { Write-Host ("  [ OK ] guile coincide en {0} programas" -f $tests.Count) }
        else { $fail += $gfail }
    } else {
        Write-Host "  [SKIP] guile no está en el PATH"
    }
} finally {
    Pop-Location
}

Write-Host ""
if ($fail -eq 0) {
    Write-Host ("RESULTADO: OK ({0} programas coinciden)" -f $pass)
    exit 0
}
Write-Host ("RESULTADO: FAIL ({0}/{1})" -f $fail, ($pass + $fail))
exit 1
