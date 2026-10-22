# build.ps1 - Compila el shim C de I/O (modules/io/io_shim.dll).
#
# Uso (desde cualquier CWD; las rutas se resuelven con $PSScriptRoot):
#   powershell -File modules\io\build.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH.

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$shimSrc = Join-Path $here "io_shim.c"
$shimDll = Join-Path $here "io_shim.dll"

Write-Host "== modules/io: build =="

$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$needBuild = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
if ($needBuild) {
    if (-not $gcc) {
        Write-Host "ERROR: falta $shimDll y no hay gcc en PATH."
        exit 2
    }
    Write-Host "== compilando shim C (io_shim.dll) =="
    & gcc -shared -O2 -o $shimDll $shimSrc
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: gcc fallo."; exit 1 }
} else {
    Write-Host "== shim C ya compilado =="
}
Write-Host "OK"
exit 0
