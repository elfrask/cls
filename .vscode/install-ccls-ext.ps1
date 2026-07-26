# Script para instalar la extensión CLS en VS Code
# Ejecutar: powershell -ExecutionPolicy Bypass .\.vscode\install-ccls-ext.ps1

$source = Join-Path $PSScriptRoot "extensions\ccls-lang"
$dest = "$env:USERPROFILE\.vscode\extensions\frask.ccls-lang-1.0.0"

if (-not (Test-Path $source)) {
    Write-Error "No se encuentra la extensión en: $source"
    exit 1
}

# Eliminar si ya existe
if (Test-Path $dest) {
    Remove-Item -Recurse -Force $dest
    Write-Host "Extensión anterior eliminada."
}

# Copiar
Copy-Item -Recurse $source $dest
Write-Host "Extensión instalada en: $dest"
Write-Host ""
Write-Host "Reinicia VS Code para que los cambios surtan efecto."
Write-Host "Luego abre cualquier archivo .ccls y verás el resaltado."
