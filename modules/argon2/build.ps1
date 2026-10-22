# build.ps1 - compila argon2_shim.dll (libargon2 estatico) para Windows.
$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$root = (Resolve-Path (Join-Path $here "..\..")).Path
$src = Join-Path $root ".resources\libargon2\phc-winner-argon2-20190702"

if (-not (Test-Path (Join-Path $src "include\argon2.h"))) {
    Write-Host "== obteniendo libargon2 =="
    & (Join-Path $root "scripts\fetch-resources.ps1") -Only libargon2
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$gcc = Get-Command gcc -ErrorAction SilentlyContinue
if (-not $gcc) {
    Write-Host "ERROR: falta gcc (MinGW-w64 x64) en PATH."
    exit 2
}

$out = Join-Path $here "argon2_shim.dll"
$files = @(
    (Join-Path $here "argon2_shim.c"),
    (Join-Path $src "src\argon2.c"),
    (Join-Path $src "src\core.c"),
    (Join-Path $src "src\ref.c"),
    (Join-Path $src "src\blake2\blake2b.c"),
    (Join-Path $src "src\thread.c"),
    (Join-Path $src "src\encoding.c")
)
& gcc -shared -O2 -o $out @files -I (Join-Path $src "include") -I (Join-Path $src "src")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "OK $out"
