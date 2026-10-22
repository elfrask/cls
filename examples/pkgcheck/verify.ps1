# verify.ps1 - Prueba #8 (pkgcheck): SHA-256 en CLS puro vs oraculo.
#
#   1. corre src/main.clsx (emite 205 digests + linea corrupt=)
#   2. oraculo Python (hashlib): reproduce los mismos inputs y compara byte a byte
#   3. (Windows) cross-check independiente: certutil -hashfile SHA256 sobre un
#      archivo generado, comparado con src/hashfile.clsx
#
# Uso:  powershell -File examples\pkgcheck\verify.ps1
# Requiere: target\debug\clx.exe. Python es necesario para el oraculo principal.

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$outDir = Join-Path $here "out"

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }

Push-Location $here
$globalFails = 0
try {
    Write-Host "== 1. digests (src/main.clsx) =="
    $digests = Join-Path $outDir "digests.txt"
    $out = & $clx run "src\main.clsx" --add-path-module=../../modules 2> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[FAIL] clx run exit=$LASTEXITCODE"
        $globalFails++
    } else {
        [System.IO.File]::WriteAllLines($digests, @($out), (New-Object System.Text.UTF8Encoding($false)))
        $n = (Get-Content $digests).Count
        Write-Host ("  {0} lineas de salida" -f $n)
    }

    Write-Host ""
    Write-Host "== 2. oraculo hashlib =="
    if (Get-Command python -ErrorAction SilentlyContinue) {
        $py = & python "tests\oracle.py" $digests 2>&1 | Out-String
        Write-Host ($py.TrimEnd())
        if ($LASTEXITCODE -ne 0) { $globalFails++ }
    } else {
        Write-Host "  [SKIP] python no disponible (hashlib no verificable)"
    }

    Write-Host ""
    Write-Host "== 3. certutil (cross-check de archivo) =="
    if (Get-Command certutil -ErrorAction SilentlyContinue) {
        $f = Join-Path $outDir "sample.txt"
        [System.IO.File]::WriteAllText($f, "CLS 2.0 pkgcheck: hash de archivo de prueba`n", (New-Object System.Text.UTF8Encoding($false)))
        $ct = (& certutil -hashfile $f SHA256 | Select-Object -Index 1).Trim() -replace ' ', ''
        $cls = (& $clx run "src\hashfile.clsx" "--add-path-module=../../modules" "--" $f 2>$null | Out-String).Trim()
        Write-Host ("  certutil: {0}" -f $ct)
        Write-Host ("  cls     : {0}" -f $cls)
        if ($ct -eq $cls -and $cls -ne "") {
            Write-Host "  [ OK ] certutil coincide"
        } else {
            Write-Host "  [FAIL] certutil DIVERGE"
            $globalFails++
        }
    } else {
        Write-Host "  [SKIP] certutil no disponible"
    }
} finally {
    Pop-Location
}

Write-Host ""
if ($globalFails -eq 0) {
    Write-Host "RESULTADO: OK (SHA-256 CLS == hashlib/certutil byte a byte)"
    exit 0
}
Write-Host "RESULTADO: FAIL ($globalFails problema(s))"
exit 1
