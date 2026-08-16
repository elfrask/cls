# run-jit.ps1 - Ejecuta los ejemplos JIT-only EXCLUSIVAMENTE con `clx run --jit`.
# El walker NO se usa aquí (es solo referencia sintáctica; se deprecará tras 2.0-dev1).
# Uso: powershell -File run-jit.ps1
$ErrorActionPreference = "Stop"
$clx = Join-Path $PSScriptRoot "..\..\target\debug\clx.exe"
$clx = (Resolve-Path $clx).Path

$examples = @(
    (Join-Path $PSScriptRoot "modules\src\main.clsx")
)

foreach ($ex in $examples) {
    Write-Host ""
    Write-Host "=============================================="
    Write-Host "  JIT: $ex"
    Write-Host "=============================================="
    & $clx run --jit $ex
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FALLO (exit $LASTEXITCODE): $ex"
        exit 1
    }
}
Write-Host ""
Write-Host "Todos los ejemplos JIT pasaron."
