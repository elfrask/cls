# generate-oracles.ps1 - Genera archivos .expect con la salida esperada (oracle)
# para cada test de la suite JIT. Compara la corrida actual con el .expect
# existente (si lo hay) y pregunta antes de sobreescribir si difiere.
#
# Migracion dev-2 (Fase 7): el script original comparaba JIT vs walker.
# Al eliminarse el walker, los .expect son ahora el oracle de referencia
# (el output conocido-correcto del JIT). Si cambias el output del JIT,
# tienes que regenerar el .expect intencionalmente.
#
# Uso: powershell -File generate-oracles.ps1 [-Force]
[CmdletBinding()]
param(
    [switch]$Force
)
$ErrorActionPreference = "Continue"
# Subir 4 niveles desde examples/audit/test-features/jit-test/ hasta cls/
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..")).Path
$clx = Join-Path $root "target\debug\clx.exe"
if (-not (Test-Path $clx)) {
    Write-Error ("No se encuentra clx.exe en: {0}" -f $clx)
    exit 1
}
$clx = (Resolve-Path $clx).Path
Write-Host ("clx: {0}" -f $clx)

function New-Oracle($clsx) {
    $name = Split-Path $clsx -Leaf
    $expect = [System.IO.Path]::ChangeExtension($clsx, ".expect")
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    $p = Start-Process -FilePath $script:clx -ArgumentList @("run", $clsx) -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $content = (Get-Content -Raw -Encoding UTF8 $tmpOut -ErrorAction SilentlyContinue) + ""
    if ($content -ne "" -and -not $content.EndsWith("`n")) { $content = $content + "`n" }
    Remove-Item $tmpOut, $tmpErr -ErrorAction SilentlyContinue

    if (Test-Path $expect) {
        $existing = Get-Content -Raw $expect
        if ($existing -eq $content) {
            Write-Host ("  [=] {0,-32} sin cambios" -f $name)
            return
        }
        if (-not $Force) {
            $ans = Read-Host ("  [?] {0} difiere del .expect actual. Regenerar? [y/N]" -f $name)
            if ($ans -ne "y" -and $ans -ne "Y") {
                Write-Host "      cancelado"
                return
            }
        }
    }
    Set-Content -Path $expect -Value $content -Encoding UTF8 -NoNewline
    Write-Host ("  [+] {0,-32} generado (exit={1}, {2} bytes)" -f $name, $p.ExitCode, $content.Length)
}

# Hacer visibles $clx y $Force dentro de la funcion.
$script:clx = $clx
$script:Force = $Force

Write-Host "== Generando oracles (JIT) =="
Write-Host ""
Write-Host "-- units/ --"
$units = Get-ChildItem (Join-Path $PSScriptRoot "units") -Filter *.clsx | Sort-Object Name
foreach ($f in $units) { New-Oracle $f.FullName }
Write-Host ""
Write-Host "-- availible/ --"
$avail = Get-ChildItem (Join-Path $PSScriptRoot "availible") -Filter *.clsx | Sort-Object Name
foreach ($f in $avail) { New-Oracle $f.FullName }
Write-Host ""
$total = (Get-ChildItem (Join-Path $PSScriptRoot 'units'),(Join-Path $PSScriptRoot 'availible') -Recurse -Filter *.expect).Count
Write-Host ("Listo. Total: {0} archivos .expect" -f $total)
