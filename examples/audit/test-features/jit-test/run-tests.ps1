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
$special = @{
    "a11.clsx"     = "requiere DLL de extension (clx_demo.dll)"
    "f64arr.clsx"  = "requiere args de app (-- ola mundo)"
    "b7.clsx"      = "requiere red (httpbin.org)"
    "synerr.clsx"  = "prueba de error de sintaxis (sin paridad)"
    "syn2.clsx"    = "prueba de error de sintaxis (sin paridad)"
    "a2min.clsx"   = "variante de tuplas (cubierto por a2)"
    "bench_fib.clsx" = "benchmark (usa now(), no hay paridad de tiempos)"
    "b8-cmx.clsx"   = "prueba CMX ref (sin paridad estable)"
}

function Run-Oracle($clsx) {
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    $p = Start-Process -FilePath $script:clx -ArgumentList @("run", $clsx) -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $content = (Get-Content -Raw -Encoding UTF8 $tmpOut -ErrorAction SilentlyContinue) + ""
    if ($content -ne "" -and -not $content.EndsWith("`n")) { $content = $content + "`n" }
    Remove-Item $tmpOut, $tmpErr -ErrorAction SilentlyContinue
    return @{ Content = $content; Exit = $p.ExitCode }
}

function Compare-To-Oracle($clsx) {
    $expect = [System.IO.Path]::ChangeExtension($clsx, ".expect")
    $r = Run-Oracle $clsx
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
    $r = Compare-To-Oracle $f.FullName
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
