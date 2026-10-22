# fetch-resources.ps1 - descarga recursos nativos externos declarados en
# resources/manifest.json. Cache local: <repo>/.resources/<recurso>/.
#
# Uso:
#   powershell -File scripts/fetch-resources.ps1
#   powershell -File scripts/fetch-resources.ps1 -Only libargon2 -Force
param(
    [string[]]$Only,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$root = (Resolve-Path (Join-Path $here "..")).Path
$manifestPath = Join-Path $root "resources\manifest.json"
$cacheRoot = Join-Path $root ".resources"

if (-not (Test-Path $manifestPath)) {
    Write-Host "ERROR: falta $manifestPath"
    exit 2
}
$manifest = Get-Content $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null

$names = @($manifest.PSObject.Properties.Name)
if ($Only -and $Only.Count -gt 0) {
    $unknown = @($Only | Where-Object { $names -notcontains $_ })
    if ($unknown.Count -gt 0) {
        Write-Host ("ERROR: recursos desconocidos: " + ($unknown -join ", "))
        exit 2
    }
    $names = @($Only)
}

foreach ($name in $names) {
    $res = $manifest.$name
    $dir = Join-Path $cacheRoot $name
    $zip = Join-Path $dir $res.archive
    $extract = Join-Path $dir $res.extract_dir
    New-Item -ItemType Directory -Path $dir -Force | Out-Null

    if ($Force -or -not (Test-Path $zip)) {
        Write-Host "descargando $name $($res.version): $($res.url)"
        Invoke-WebRequest -UseBasicParsing -Uri $res.url -OutFile $zip
    }

    $got = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    $want = $res.sha256.ToLowerInvariant()
    if ($got -ne $want) {
        Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
        Write-Host "ERROR: sha256 de $name no coincide"
        Write-Host "  esperado: $want"
        Write-Host "  obtenido: $got"
        exit 1
    }

    if ($Force -or -not (Test-Path $extract)) {
        if (Test-Path $extract) { Remove-Item -Recurse -Force $extract }
        Write-Host "extrayendo $name..."
        Expand-Archive -LiteralPath $zip -DestinationPath $dir -Force
    }
    Write-Host "OK $name -> $extract"
}
