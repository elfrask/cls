# run-availible.ps1 - Ejecuta cada test de la carpeta availible/ y compara
# su salida contra el oracle (.expect). Cualquier diferencia aparece como FAIL.
#
# Migracion dev-2 (Fase 7): el script original comparaba JIT vs walker.
# Al eliminarse el walker, los .expect son ahora el oracle de referencia
# (output conocido-correcto del JIT).
#
# Uso: powershell -File run-availible.ps1 [-Update]
#   -Update   Regenera los .expect cuando difieren (sin preguntar).
[CmdletBinding()]
param(
    [switch]$Update
)
$ErrorActionPreference = "Continue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
$clx = (Resolve-Path (Join-Path $root "target\debug\clx.exe")).Path
$dir = Join-Path $PSScriptRoot "availible"
$script:clx = $clx
$script:Update = $Update

# Tests que no se ejecutan como programa:
# - libmod.clsx: es un modulo con exports (incluido por 22-include), no
#   tiene function main. No es un test, es una dependencia.
$preExisting = @{
    "libmod.clsx" = "modulo (incluido por 22-include), no tiene main"
}

function Run-Oracle($clsx) {
    $name = Split-Path $clsx -Leaf
    $expect = [System.IO.Path]::ChangeExtension($clsx, ".expect")
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
    $name = Split-Path $clsx -Leaf
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
Write-Host "== Features disponibles del JIT (paridad vs oracle) =="
Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f "SCRIPT", "RES", "EXIT", "SALIDA (JIT) / NOTA")
Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f "------", "---", "----", "-----")
$files = Get-ChildItem $dir -Filter *.clsx | Sort-Object Name
foreach ($f in $files) {
    if ($preExisting.ContainsKey($f.Name)) {
        Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f $f.Name, "SKIP", "-", $preExisting[$f.Name])
        $skip++
        continue
    }
    $r = Compare-To-Oracle $f.FullName
    $primera = (($r.Output -split "`r?`n" | Where-Object { $_ -ne "" }) | Select-Object -First 1)
    if (-not $primera) { $primera = "(sin salida)" }
    switch ($r.Status) {
        "PASS"    { $pass++; Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f $f.Name, "PASS", $r.Exit, $primera) }
        "FAIL"    { $fail++; Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f $f.Name, "FAIL", $r.Exit, $primera) }
        "NEW"     { $new++;  Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f $f.Name, "NEW",  $r.Exit, $primera) }
        "UPDATED" { $pass++; Write-Host ("{0,-26} {1,-8} {2,-6} {3}" -f $f.Name, "UPDT", $r.Exit, $primera) }
    }
}
Write-Host ""
Write-Host ("Resultado: {0} PASS, {1} FAIL, {2} SKIP, {3} NEW" -f $pass, $fail, $skip, $new)
if ($fail -gt 0) { exit 1 } else { exit 0 }
