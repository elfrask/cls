# build.ps1 - compila el shim FFI de la prueba (langffi_shim.dll).
# Uso: powershell -File examples\lang-features\ffi\build.ps1
$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$src = Join-Path $here "langffi_shim.c"
$dll = Join-Path $here "langffi_shim.dll"
$gcc = Get-Command gcc -ErrorAction SilentlyContinue
if (-not $gcc) {
    Write-Host "ERROR: no se encontro 'gcc' en PATH."
    exit 2
}
& gcc -shared -O2 -o $dll $src
if ($LASTEXITCODE -ne 0) { exit 1 }
Write-Host "OK: $dll"
