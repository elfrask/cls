$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$src = Join-Path $here "random_shim.c"
$dll = Join-Path $here "random_shim.dll"
gcc -shared -O2 -o $dll $src -lbcrypt
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "OK $dll"
