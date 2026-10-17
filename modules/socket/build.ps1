# build.ps1 - Compila el shim C de sockets (modules/socket/socket_shim.dll).
#
# Uso (desde cualquier CWD; las rutas se resuelven con $PSScriptRoot):
#   powershell -File modules\socket\build.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH. En Windows enlaza ws2_32.

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$shimSrc = Join-Path $here "socket_shim.c"
$shimDll = Join-Path $here "socket_shim.dll"

Write-Host "== modules/socket: build =="

$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$needBuild = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
if ($needBuild) {
    if (-not $gcc) {
        Write-Host "ERROR: falta $shimDll y no hay gcc en PATH."
        exit 2
    }
    Write-Host "== compilando shim C (socket_shim.dll) =="
    & gcc -shared -O2 -o $shimDll $shimSrc -lws2_32
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: gcc fallo."; exit 1 }
} else {
    Write-Host "== shim C ya compilado =="
}
Write-Host "OK"
exit 0
