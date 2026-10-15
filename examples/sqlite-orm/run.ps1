# run.ps1 - Runner diferencial de la prueba #5 (SQLite ORM).
#
#   1. compila el shim C `oracle/sqlite_shim.dll` (si falta o esta viejo)
#   2. localiza `sqlite3.dll` (runtime de Python) y lo expone al shim
#   3. corre el runner CLS (ORM)  -> out/cls.db + CHECK de lectura
#   4. corre el oraculo Python    -> out/py.db + CHECK de referencia
#   5. compara BYTE A BYTE:
#        - las lineas CHECK de CLS vs Python (lectura via ORM)
#        - dump(cls.db) vs dump(py.db)     (contenido/tipos/UTF-8/orden)
#        - integrity_check(cls.db) == ok
#
# Falla (exit != 0) si CUALQUIER comparacion diverge.
#
# Uso (desde la raiz del repo):
#   powershell -File examples\sqlite-orm\run.ps1
#
# Requiere: gcc (MinGW-w64 x64) en PATH, Python 3 en PATH y
# target\debug\clx.exe compilado. CWD = raiz del repo (el .dll del shim FFI se
# resuelve relativo al CWD; ver README).

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$oraclePy = Join-Path $here "oracle\oracle.py"
$shimSrc = Join-Path $here "oracle\sqlite_shim.c"
$shimDll = Join-Path $here "oracle\sqlite_shim.dll"
$outDir = Join-Path $here "out"
$clsDb = Join-Path $outDir "cls.db"
$pyDb = Join-Path $outDir "py.db"

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { Write-Host "ERROR: no se encontro 'python' en PATH (oraculo)."; exit 2 }

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# --- sqlite3.dll del runtime de Python ---
$sqliteDll = (& python -c "import sys,os; p=os.path.join(sys.prefix,'DLLs','sqlite3.dll'); print(p if os.path.exists(p) else '')").Trim()
if ($sqliteDll -ne "") {
    $env:CLS_SQLITE_PATH = $sqliteDll
    $localCopy = Join-Path $here "oracle\sqlite3.dll"
    if (-not (Test-Path $localCopy)) { Copy-Item $sqliteDll $localCopy -Force -ErrorAction SilentlyContinue }
    Write-Host "sqlite3.dll: $sqliteDll"
} else {
    Write-Host "AVISO: no se encontro sqlite3.dll via Python; el shim intentara PATH."
}

# --- shim C ---
$gcc = Get-Command gcc -ErrorAction SilentlyContinue
$needBuild = (-not (Test-Path $shimDll)) -or ((Get-Item $shimSrc).LastWriteTime -gt (Get-Item $shimDll).LastWriteTime)
if ($needBuild) {
    if (-not $gcc) { Write-Host "ERROR: falta $shimDll y no hay gcc."; exit 2 }
    Write-Host "== compilando shim C (sqlite_shim.dll) =="
    & gcc -shared -O2 -o $shimDll $shimSrc
    if ($LASTEXITCODE -ne 0) { exit 1 }
} else {
    Write-Host "== shim C ya compilado =="
}

function Compare-Bytes([string]$a, [string]$b, [string]$label) {
    $ba = [System.IO.File]::ReadAllBytes($a)
    $bb = [System.IO.File]::ReadAllBytes($b)
    if ($ba.Length -ne $bb.Length) {
        Write-Host ("  [FAIL] {0}: longitudes {1} vs {2}" -f $label, $ba.Length, $bb.Length)
        return $false
    }
    for ($i = 0; $i -lt $ba.Length; $i++) {
        if ($ba[$i] -ne $bb[$i]) {
            Write-Host ("  [FAIL] {0}: primer byte distinto en {1}" -f $label, $i)
            return $false
        }
    }
    Write-Host ("  [ OK ] {0}: {1} bytes identicos" -f $label, $ba.Length)
    return $true
}

Write-Host ""
Write-Host "=== SQLite ORM: CLS (ORM) vs oraculo Python sqlite3 (diferencial byte a byte) ==="
Write-Host ""

# --- 1. CLS (ORM) ---
Write-Host "== corriendo runner CLS (ORM) =="
Push-Location $repo
& $clx run "examples\sqlite-orm\src\main.clsx" 1> (Join-Path $outDir "cls_checks.txt") 2> (Join-Path $outDir "cls_stderr.txt")
$clsExit = $LASTEXITCODE
Pop-Location
if ($clsExit -ne 0) {
    Write-Host "ERROR: el runner CLS fallo (exit=$clsExit). stderr:"
    Get-Content (Join-Path $outDir "cls_stderr.txt") | Select-Object -Last 20
    exit 1
}
if (-not (Test-Path $clsDb)) { Write-Host "ERROR: no se genero $clsDb"; exit 1 }

# --- 2. Oraculo Python ---
Write-Host "== corriendo oraculo Python (construye py.db) =="
& python $oraclePy run $pyDb 1> (Join-Path $outDir "py_checks.txt") 2> (Join-Path $outDir "py_stderr.txt")
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: el oraculo Python fallo. stderr:"
    Get-Content (Join-Path $outDir "py_stderr.txt") | Select-Object -Last 20
    exit 1
}

# --- 3. Comparaciones ---
Write-Host ""
Write-Host "== comparando =="
$ok = $true
$ok = (Compare-Bytes (Join-Path $outDir "cls_checks.txt") (Join-Path $outDir "py_checks.txt") "CHECK (lectura ORM vs Python)") -and $ok

& python $oraclePy dump $clsDb 1> (Join-Path $outDir "cls_dump.txt") 2>$null
& python $oraclePy dump $pyDb  1> (Join-Path $outDir "py_dump.txt")  2>$null
$ok = (Compare-Bytes (Join-Path $outDir "cls_dump.txt") (Join-Path $outDir "py_dump.txt") "dump(cls.db) vs dump(py.db)") -and $ok

$integ = (& python $oraclePy integrity $clsDb).Trim()
if ($integ -eq "ok") {
    Write-Host "  [ OK ] PRAGMA integrity_check(cls.db) = ok"
} else {
    Write-Host "  [FAIL] PRAGMA integrity_check(cls.db) = '$integ'"
    $ok = $false
}

Write-Host ""
if ($ok) {
    Write-Host "RESULTADO: OK (ORM CLS == oraculo Python byte a byte; integridad ok)"
    exit 0
}
Write-Host "RESULTADO: DIVERGENCIA (ver arriba)"
exit 1
