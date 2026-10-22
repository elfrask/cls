# run.ps1 - Suite de la prueba CMX templates (examples/cmx-templates).
#
#   1. renderiza src/main.clsx -> out/*.html (CLS puro)
#   2. compara cada out/*.html byte a byte (normalizado LF) contra tests/golden/
#      (los golden son el oraculo: sin walker, la salida esperada esta versionada)
#   3. oraculo cruzado Jinja2 (tests/oracle/render_jinja.py) sobre la paridad
#   4. corre tests/findings/*.clsx: cada repro DEBE fallar (exit != 0 = bug
#      PRESENTE); gaps.txt lista los GAPs documentados (no cuentan)
#
# Uso:  powershell -File examples\cmx-templates\run.ps1 [-Update]
#   -Update   regenera tests/golden/ con la salida actual (revisar antes de commitear).
#
# Requiere: target\debug\clx.exe compilado. Jinja2 opcional (pip install jinja2).

[CmdletBinding()]
param(
    [switch]$Update
)

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$goldenDir = Join-Path $here "tests\golden"
$outDir = Join-Path $here "out"
$findDir = Join-Path $here "tests\findings"
$oracle = Join-Path $here "tests\oracle\render_jinja.py"

function Run-Clx {
    param([string[]]$ClxArgs)
    $o = [System.IO.Path]::GetTempFileName()
    $e = [System.IO.Path]::GetTempFileName()
    & $script:clx @ClxArgs 1> $o 2> $e
    $code = $LASTEXITCODE
    $text = ((Get-Content -Raw -Encoding UTF8 $o -ErrorAction SilentlyContinue) + (Get-Content -Raw -Encoding UTF8 $e -ErrorAction SilentlyContinue))
    Remove-Item $o, $e -ErrorAction SilentlyContinue
    if ($null -eq $text) { $text = "" }
    return [pscustomobject]@{ Text = $text; Code = $code }
}

function Read-Norm {
    param([string]$Path)
    $t = Get-Content -Raw -Encoding UTF8 $Path -ErrorAction SilentlyContinue
    if ($null -eq $t) { $t = "" }
    return ($t -replace "`r`n", "`n").TrimEnd()
}

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

Push-Location $here
$globalFails = 0
try {
    # --- 1. render ---
    Write-Host "== 1. render (src/main.clsx -> out/) =="
    if (Test-Path $outDir) { Remove-Item -Recurse -Force $outDir }
    $r = Run-Clx @("run", "src\main.clsx", "--add-path-module=../../modules")
    Write-Host $r.Text.TrimEnd()
    if ($r.Code -ne 0) {
        Write-Host "[FAIL] render exit=$($r.Code)"
        $globalFails++
        Pop-Location
        exit 1
    }

    # --- 2. goldens ---
    Write-Host ""
    Write-Host "== 2. paginas vs tests/golden/ =="
    $outs = @(Get-ChildItem $outDir -Filter *.html -ErrorAction SilentlyContinue | Sort-Object Name)
    if ($Update -or -not (Test-Path $goldenDir)) {
        New-Item -ItemType Directory -Force -Path $goldenDir | Out-Null
        foreach ($o in $outs) { Copy-Item $o.FullName (Join-Path $goldenDir $o.Name) -Force }
        Write-Host "goldens actualizados ($($outs.Count))"
    }
    $goldens = @(Get-ChildItem $goldenDir -Filter *.html -ErrorAction SilentlyContinue | Sort-Object Name)
    foreach ($o in $outs) {
        $g = Join-Path $goldenDir $o.Name
        if (-not (Test-Path $g)) {
            Write-Host ("  [FAIL] {0}: falta golden" -f $o.Name)
            $globalFails++
            continue
        }
        $a = Read-Norm $o.FullName
        $b = Read-Norm $g
        if ($a -eq $b) {
            Write-Host ("  [ OK ] {0}" -f $o.Name)
        } else {
            Write-Host ("  [FAIL] {0}: DIVERGE del golden" -f $o.Name)
            Write-Host ("         cls  : {0}" -f $a)
            Write-Host ("         golden: {0}" -f $b)
            $globalFails++
        }
    }
    foreach ($g in $goldens) {
        if (-not (Test-Path (Join-Path $outDir $g.Name))) {
            Write-Host ("  [FAIL] {0}: golden huerfano (ya no se genera)" -f $g.Name)
            $globalFails++
        }
    }

    # --- 3. oraculo Jinja2 ---
    Write-Host ""
    Write-Host "== 3. oraculo Jinja2 (paridad) =="
    if (Get-Command python -ErrorAction SilentlyContinue) {
        $py = & python $oracle 2>&1 | Out-String
        $pyCode = $LASTEXITCODE
        Write-Host ("  " + $py.Trim())
        if ($pyCode -eq 0) {
            Write-Host "  [ OK ] Jinja2 coincide"
        } elseif ($pyCode -eq 3) {
            Write-Host "  [SKIP] Jinja2 no disponible"
        } else {
            Write-Host "  [FAIL] Jinja2 DIVERGE"
            $globalFails++
        }
    } else {
        Write-Host "  [SKIP] python no disponible"
    }

    # --- 4. findings ---
    Write-Host ""
    Write-Host "== 4. findings / repros (exit != 0 = BUG PRESENTE) =="
    $gapFile = Join-Path $findDir "gaps.txt"
    $gapNames = @{}
    if (Test-Path $gapFile) {
        foreach ($ln in Get-Content $gapFile) {
            $t = $ln.Trim()
            if ($t -ne "" -and -not $t.StartsWith("#")) { $gapNames[$t] = $true }
        }
    }
    $openBugs = 0
    $resolved = 0
    $gaps = 0
    $findings = @(Get-ChildItem $findDir -Filter *.clsx -ErrorAction SilentlyContinue | Sort-Object Name)
    foreach ($f in $findings) {
        if ($gapNames.ContainsKey($f.Name)) {
            $gaps++
            Write-Host ("  [GAP]  {0}" -f $f.Name)
            continue
        }
        $fr = Run-Clx @("run", $f.FullName, "--add-path-module=../../modules")
        if ($fr.Code -ne 0) {
            $openBugs++
            Write-Host ("  [BUG]  {0}" -f $f.Name)
        } else {
            $resolved++
            Write-Host ("  [FIX?] {0} ya no falla -> promover a la matriz" -f $f.Name)
        }
    }

    # --- 5. resumen ---
    Write-Host ""
    Write-Host "===================== RESUMEN ====================="
    Write-Host ("paginas:           {0} golden(s)" -f $goldens.Count)
    Write-Host ("findings abiertos: {0}" -f $openBugs)
    Write-Host ("findings resueltos: {0}" -f $resolved)
    Write-Host ("findings GAP (no regresion): {0}" -f $gaps)
    if ($openBugs -gt 0) { $globalFails++ }
    Write-Host "==================================================="
} finally {
    Pop-Location
}

if ($globalFails -eq 0) {
    Write-Host "RESULTADO GLOBAL: OK"
    exit 0
}
Write-Host "RESULTADO GLOBAL: FAIL ($globalFails problema(s))"
exit 1
