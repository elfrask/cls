# build.ps1 - Compila el shim C de SQLite (modules/sqlite) y prepara sqlite3.dll.
#
#   1. localiza `sqlite3.dll` (runtime de Python) y lo COPIA junto al shim
#   2. exporta CLS_SQLITE_PATH para esta sesion apuntando a esa copia
#   3. compila `sqlite_shim.dll` (si falta o esta viejo) desde `sqlite_shim.c`
#
# Uso (desde cualquier CWD; las rutas se resuelven con $PSScriptRoot):
#   powershell -File modules\sqlite\build.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH y Python 3 en PATH (para sqlite3.dll).
# El shim autolocaliza sqlite3.dll junto a el, por lo que la copia basta para
# consumirlo; CLS_SQLITE_PATH queda como respaldo.

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$shimSrc = Join-Path $here "sqlite_shim.c"
$shimDll = Join-Path $here "sqlite_shim.dll"
$localSqlite = Join-Path $here "sqlite3.dll"

Write-Host "== modules/sqlite: build =="

# --- 1/2. sqlite3.dll del runtime de Python -> junto al shim ---
$python = Get-Command python -ErrorAction SilentlyContinue
$sqliteDll = ""
if ($python) {
    $sqliteDll = (& python -c "import sys,os; p=os.path.join(sys.prefix,'DLLs','sqlite3.dll'); print(p if os.path.exists(p) else '')").Trim()
}
if ($sqliteDll -ne "") {
    Copy-Item $sqliteDll $localSqlite -Force
    $env:CLS_SQLITE_PATH = $localSqlite
    Write-Host "sqlite3.dll: $localSqlite (copia de $sqliteDll)"
    Write-Host "CLS_SQLITE_PATH=$localSqlite"
} else {
    Write-Host "AVISO: no se encontro sqlite3.dll via Python; el shim intentara PATH."
}

# --- 3. shim C ---
$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$needBuild = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
if ($needBuild) {
    if (-not $gcc) {
        Write-Host "ERROR: falta $shimDll y no hay gcc en PATH."
        exit 2
    }
    Write-Host "== compilando shim C (sqlite_shim.dll) =="
    & gcc -shared -O2 -o $shimDll $shimSrc
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: gcc fallo."; exit 1 }
} else {
    Write-Host "== shim C ya compilado =="
}
Write-Host "OK"
