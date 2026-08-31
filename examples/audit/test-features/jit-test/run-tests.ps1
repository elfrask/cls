# run-tests.ps1 - Ejecuta cada test de units/ y compara su salida contra
# el oracle (.expect). Cualquier diferencia aparece como FAIL.
#
# Migracion dev-2 (Fase 7): el script original comparaba JIT vs walker.
# Al eliminarse el walker, los .expect son ahora el oracle de referencia
# (output conocido-correcto del JIT).
#
# Tests marcados como "special" se SKIPean: requieren condiciones
# externas (red, DLLs, args especiales) o son pruebas de error que
# comparan su salida en stderr (no en stdout).
#
# Uso: powershell -File run-tests.ps1 [-Update]
#   -Update   Regenera los .expect cuando difieren (sin preguntar).
[CmdletBinding()]
param(
    [switch]$Update
)
$ErrorActionPreference = "Continue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
$clx = (Resolve-Path (Join-Path $root "target\debug\clx.exe")).Path
$units = Join-Path $PSScriptRoot "units"
$script:clx = $clx
$script:Update = $Update

# Scripts que requieren condiciones especiales (no son fallos).
# Migracion dev-2 (Fase 7+8): a2min, b8-cmx, f64arr, a11 y bench_fib
# ahora tienen oracle y pasan. b7 (red) se valida por regex (ver rangeChecks).
$special = @{}

# Tests que DEBEN fallar (errores de sintaxis intencionales). El runner
# valida que el comando sale con codigo != 0 y produce stderr.
$expectedErrors = @{
    "synerr.clsx" = "error de sintaxis intencional"
    "syn2.clsx"   = "error de sintaxis intencional"
}

# Args de app por test (se pasan tras `--`). Migracion dev-2 (Fase 8):
# f64arr.clsx imprime `print("test", args)` y espera recibir ["ola", "mundo"].
$appArgs = @{
    "f64arr.clsx" = @("ola", "mundo")
}

# Tests con salida no deterministica (tiempos, etc.) validados por regex
# en lugar de oracle exacto. Migracion dev-2 (Fase 8): bench_fib imprime
# fib(26) (deterministico: 121393) + ms (no deterministico). b7 (red)
# imprime el len de la respuesta de httpbin (varia) + flags estables.
# Ambos se validan por regex. Si b7 falla por red, se cuenta como SKIP.
$rangeChecks = @{
    "bench_fib.clsx" = "fib\(26\): 121393"
    "b7.clsx"        = "http ok: true"
}

function Run-Oracle($clsx, $extraArgs) {
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    $argList = @("run", $clsx)
    if ($extraArgs -and $extraArgs.Count -gt 0) {
        $argList += "--"
        $argList += $extraArgs
    }
    $p = Start-Process -FilePath $script:clx -ArgumentList $argList -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $content = (Get-Content -Raw -Encoding UTF8 $tmpOut -ErrorAction SilentlyContinue) + ""
    if ($content -ne "" -and -not $content.EndsWith("`n")) { $content = $content + "`n" }
    $err = (Get-Content -Raw -Encoding UTF8 $tmpErr -ErrorAction SilentlyContinue) + ""
    Remove-Item $tmpOut, $tmpErr -ErrorAction SilentlyContinue
    return @{ Content = $content; Err = $err; Exit = $p.ExitCode }
}

function Compare-To-Oracle($clsx, $extraArgs) {
    $expect = [System.IO.Path]::ChangeExtension($clsx, ".expect")
    $r = Run-Oracle $clsx $extraArgs
    if (-not (Test-Path $expect)) {
        Set-Content -Path $expect -Value $r.Content -Encoding UTF8 -NoNewline
        return @{ Status = "NEW"; Output = $r.Content; Exit = $r.Exit }
    }
    $existing = Get-Content -Raw $expect
    if ($existing -eq $r.Content) {
        return @{ Status = "PASS"; Output = $r.Content; Exit = $r.Exit }
    }
    if ($script:Update) {
        Set-Content -Path $expect -Value $r.Content -Encoding UTF8 -NoNewline
        return @{ Status = "UPDATED"; Output = $r.Content; Exit = $r.Exit }
    }
    return @{ Status = "FAIL"; Output = $r.Content; Exit = $r.Exit }
}

$pass = 0; $fail = 0; $skip = 0; $new = 0
Write-Host "== Test de features JIT (paridad vs oracle) =="
Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f "SCRIPT", "RES", "EXIT", "NOTA")
Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f "------", "---", "----", "----")
$files = Get-ChildItem $units -Filter *.clsx | Sort-Object Name
foreach ($f in $files) {
    $name = $f.Name
    if ($special.ContainsKey($name)) {
        Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "SKIP", "-", $special[$name])
        $skip++
        continue
    }
    # Test que DEBE fallar: validar exit != 0 y stderr no vacio.
    if ($expectedErrors.ContainsKey($name)) {
        $r = Run-Oracle $f.FullName
        if ($r.Exit -ne 0 -and -not [string]::IsNullOrWhiteSpace($r.Err)) {
            $pass++
            $errLine = ($r.Err -split "`r?`n" | Where-Object { $_.Trim() -ne "" } | Select-Object -First 1)
            Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "PASS", $r.Exit, $errLine)
        } else {
            $fail++
            Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "FAIL", $r.Exit, "esperaba error de sintaxis pero no fallo")
        }
        continue
    }
    $extra = if ($appArgs.ContainsKey($name)) { $appArgs[$name] } else { @() }
    # Test con salida no deterministica: validar por regex.
    if ($rangeChecks.ContainsKey($name)) {
        $r = Run-Oracle $f.FullName $extra
        if ($r.Content -match $rangeChecks[$name]) {
            $pass++
            Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "PASS", $r.Exit, "regex: $($rangeChecks[$name])")
        } elseif ($name -eq "b7.clsx" -and ($r.Err -match "red|connect|timeout|resolve|internet")) {
            $skip++
            Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "SKIP", $r.Exit, "sin red (b7 es smoke)")
        } else {
            $fail++
            Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "FAIL", $r.Exit, "regex no matcheo: $($rangeChecks[$name])")
        }
        continue
    }
    $r = Compare-To-Oracle $f.FullName $extra
    switch ($r.Status) {
        "PASS"    { $pass++; Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "PASS", $r.Exit, "") }
        "FAIL"    { $fail++; $primera = (($r.Output -split "`r?`n" | Where-Object { $_ -ne "" }) | Select-Object -First 1); Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "FAIL", $r.Exit, $primera) }
        "NEW"     { $new++;  Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "NEW",  $r.Exit, "oracle creado") }
        "UPDATED" { $pass++; Write-Host ("{0,-14} {1,-8} {2,-6} {3}" -f $name, "UPDT", $r.Exit, "oracle actualizado") }
    }
}
Write-Host ""
Write-Host ("Resultado: {0} PASS, {1} FAIL, {2} SKIP, {3} NEW" -f $pass, $fail, $skip, $new)
if ($fail -gt 0) { exit 1 } else { exit 0 }
