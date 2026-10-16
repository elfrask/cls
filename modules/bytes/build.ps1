# build.ps1 - Compila el shim C de bytes (modules/bytes/bytes_shim.dll).
#
# Uso (desde cualquier CWD; las rutas se resuelven con $PSScriptRoot):
#   powershell -File modules\bytes\build.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH.

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$shimSrc = Join-Path $here "bytes_shim.c"
$shimDll = Join-Path $here "bytes_shim.dll"

Write-Host "== modules/bytes: build =="

$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$needBuild = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
if ($needBuild) {
    if (-not $gcc) {
        Write-Host "ERROR: falta $shimDll y no hay gcc en PATH."
        exit 2
    }
    Write-Host "== compilando shim C (bytes_shim.dll) =="
    & gcc -shared -O2 -o $shimDll $shimSrc
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: gcc fallo."; exit 1 }
} else {
    Write-Host "== shim C ya compilado =="
}
Write-Host "OK"
exit 0
