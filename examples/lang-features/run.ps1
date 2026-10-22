# run.ps1 - Matriz de features del lenguaje CLS 2.0 (examples/lang-features).
#
#   1. (re)compila el shim FFI (ffi/langffi_shim.dll) si falta o esta viejo
#   2. corre src/main.clsx y compara su salida byte-a-byte contra expected.txt
#      (golden). Cualquier divergencia = regresion.
#   3. corre negative/*.clsx: cada acceso invalido DEBE ser rechazado (exit != 0).
#   4. corre findings/*.clsx: cada repro DEBE fallar (exit != 0 = bug PRESENTE);
#      si alguno pasa, el bug fue arreglado -> promover a la matriz. Los
#      archivos listados en findings/gaps.txt son GAPs documentados (no cuentan).
#   5. resume y devuelve exit != 0 si la matriz diverge, si un negativo no es
#      rechazado, o si quedan findings (bugs) abiertos.
#
# Uso:  powershell -File examples\lang-features\run.ps1 [-Update]
#   -Update   regenera expected.txt con la salida actual (revisar antes de commitear).
#
# Requiere: target\debug\clx.exe compilado y 'gcc' en PATH (para el shim FFI).

[CmdletBinding()]
param(
    [switch]$Update
)

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$proj = "examples\lang-features"
$main = "examples\lang-features\src\main.clsx"
$expected = Join-Path $here "expected.txt"

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

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

Push-Location $repo
$globalFails = 0
try {
    # --- 1. shim FFI ---
    $shimSrc = Join-Path $here "ffi\langffi_shim.c"
    $shimDll = Join-Path $here "ffi\langffi_shim.dll"
    $needShim = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
    if ($needShim) {
        $gcc = Get-Command gcc -ErrorAction SilentlyContinue
        if (-not $gcc) { Write-Host "ERROR: falta 'gcc' para compilar el shim FFI."; Pop-Location; exit 2 }
        Write-Host "== compilando shim FFI (langffi_shim.dll) =="
        & gcc -shared -O2 -o $shimDll $shimSrc
        if ($LASTEXITCODE -ne 0) { Pop-Location; exit 1 }
    } else {
        Write-Host "== shim FFI ya compilado =="
    }

    # --- 2. matriz vs expected.txt ---
    Write-Host ""
    Write-Host "== 1. matriz de features (src/main.clsx) vs expected.txt =="
    $r = Run-Clx @("run", $main, "--add-path-module=modules")
    $got = $r.Text.TrimEnd()
    if ($Update -or -not (Test-Path $expected)) {
        Set-Content -Path $expected -Value $got -Encoding UTF8 -NoNewline
        Write-Host "expected.txt actualizado ($($got.Length) chars)."
    }
    $exp = (Get-Content -Raw $expected).TrimEnd()
    if ($got -eq $exp) {
        Write-Host "[ OK ] matriz: salida identica al golden"
    } else {
        Write-Host "[FAIL] matriz: la salida DIVERGE de expected.txt"
        $globalFails++
        $gl = $got -split "`r?`n"
        $el = $exp -split "`r?`n"
        $n = [Math]::Max($gl.Count, $el.Count)
        for ($i = 0; $i -lt $n; $i++) {
            $a = if ($i -lt $gl.Count) { $gl[$i] } else { "<falta>" }
            $b = if ($i -lt $el.Count) { $el[$i] } else { "<falta>" }
            if ($a -ne $b) { Write-Host ("  linea {0}: JIT='{1}' golden='{2}'" -f ($i+1), $a, $b) }
        }
    }
    if ($r.Code -ne 0) {
        Write-Host "[FAIL] matriz: exit=$($r.Code) (features fallidas en la salida)"
        $globalFails++
    }

    # --- 3. negativos (deben ser rechazados) ---
    Write-Host ""
    Write-Host "== 2. accesos invalidos (deben ser RECHAZADOS, exit != 0) =="
    $negDir = Join-Path $here "negative"
    $negs = Get-ChildItem $negDir -Filter *.clsx | Sort-Object Name
    foreach ($f in $negs) {
        $nr = Run-Clx @("run", $f.FullName)
        if ($nr.Code -ne 0) {
            Write-Host ("  [OK]   {0} (rechazado)" -f $f.Name)
        } else {
            Write-Host ("  [FAIL] {0} compilo (deberia ser rechazado)" -f $f.Name)
            $globalFails++
        }
    }

    # --- 4. findings (deben fallar mientras el bug exista) ---
    Write-Host ""
    Write-Host "== 3. findings / repros (exit != 0 = BUG PRESENTE) =="
    $findDir = Join-Path $here "findings"
    $findings = Get-ChildItem $findDir -Filter *.clsx | Sort-Object Name
    # GAPs documentados (features no implementadas / limite de diseno): no son
    # regresiones. Se listan en findings/gaps.txt (una linea por archivo).
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
    foreach ($f in $findings) {
        if ($gapNames.ContainsKey($f.Name)) {
            $gaps++
            Write-Host ("  [GAP]  {0} (feature no implementada / limite documentado)" -f $f.Name)
            continue
        }
        $fr = Run-Clx @("run", $f.FullName, "--add-path-module=modules")
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
    Write-Host ("matriz:            {0}" -f $(if ((Get-Content -Raw $expected).TrimEnd() -eq $got -and $r.Code -eq 0) { "OK" } else { "DIVERGE" }))
    Write-Host ("negativos:         {0}/{1} rechazados" -f (($negs | Where-Object { (Run-Clx @('run', $_.FullName)).Code -ne 0 }).Count, $negs.Count))
    Write-Host ("findings abiertos: {0}" -f $openBugs)
    Write-Host ("findings resueltos: {0}" -f $resolved)
    Write-Host ("findings GAP (no regresion): {0}" -f $gaps)
    # Solo los findings ABIERTOS son fallo. Un repro que pasa = bug arreglado
    # (se sugiere promoverlo a la matriz; ya no es una regresion abierta).
    if ($openBugs -gt 0) { $globalFails++ }
    Write-Host "==================================================="
}
finally {
    Pop-Location
}

if ($globalFails -eq 0) {
    Write-Host "RESULTADO GLOBAL: OK"
    exit 0
}
Write-Host "RESULTADO GLOBAL: FAIL ($globalFails problema(s))"
exit 1
