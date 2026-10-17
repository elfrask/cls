# run.ps1 - Runner diferencial de la prueba #6 (DNS resolver/authoritative).
#
# Orquesta, desde la raiz del repo:
#   1. compila los shims FFI (modules/net + modules/io)
#   2. genera los vectores y lo esperado con el oraculo Python (vector.py gen)
#   3. OFFLINE: el runner CLS parsea los vectores -> CHECK; ademas construye las
#      queries canonicas (builder) y prueba answer() por round-trip.
#        - compara CHECK CLS vs esperado Python
#        - compara cls_queries.hex vs queries_expected.txt (BUILDER byte a byte)
#   4. LIVE: levanta el servidor autoritativo Python (UDP+TCP), corre el runner
#      CLS (UDP + retry TCP si TC), lo corta, y compara:
#        - CHECK CLS vs live_expected.txt
#        - queries.log (lo recibido) vs live_queries_expected.txt
#   5. OPCIONAL: nslookup contra el server local (SKIP si no esta).
#
# Falla (exit != 0) si CUALQUIER comparacion diverge.
#
# Uso (desde la raiz del repo):
#   powershell -File examples\dns\run.ps1

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$main = Join-Path $here "src\main.clsx"
$vectorPy = Join-Path $here "oracle\vector.py"
$serverPy = Join-Path $here "oracle\server.py"
$outDir = Join-Path $here "out"

if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { Write-Host "ERROR: no se encontro 'python' en PATH (oraculo)."; exit 2 }

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

function Compare-Text([string]$a, [string]$b, [string]$label) {
    # Comparacion ESTRICTA: case-sensitive y sensible al orden (no Compare-Object,
    # que ignora mayusculas y reordena). Solo se normalizan los CR (CRLF vs LF).
    $ta = ([System.IO.File]::ReadAllText($a)) -replace "`r", ""
    $tb = ([System.IO.File]::ReadAllText($b)) -replace "`r", ""
    $la = $ta.TrimEnd("`n") -split "`n"
    if ($ta -ceq $tb) {
        Write-Host ("  [ OK ] {0}: {1} lineas identicas" -f $label, $la.Count)
        return $true
    }
    Write-Host ("  [FAIL] {0}: divergencia (orden/mayusculas importan)" -f $label)
    $lb = $tb -split "`n"
    $n = [Math]::Max($la.Count, $lb.Count)
    $shown = 0
    for ($i = 0; $i -lt $n -and $shown -lt 8; $i++) {
        $x = if ($i -lt $la.Count) { $la[$i] } else { "<faltante>" }
        $y = if ($i -lt $lb.Count) { $lb[$i] } else { "<faltante>" }
        if ($x -cne $y) {
            if ($x.Length -gt 64) { $x = $x.Substring(0,64) + "..." }
            if ($y.Length -gt 64) { $y = $y.Substring(0,64) + "..." }
            Write-Host ("         linea {0}: CLS='{1}' EXP='{2}'" -f ($i + 1), $x, $y)
            $shown++
        }
    }
    return $false
}

function Read-ResourceText([string]$path) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    return [System.Text.Encoding]::ASCII.GetString($bytes)
}

$ok = $true
Write-Host ""
Write-Host "=== DNS #6: CLS vs oraculo Python independiente (diferencial) ==="
Write-Host ""

# --- 1. shims FFI ---
Write-Host "== compilando shims FFI =="
& (Join-Path $repo "modules\net\build.ps1") ; if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: net build"; exit 1 }
& (Join-Path $repo "modules\io\build.ps1")  ; if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: io build"; exit 1 }

# --- 2. vectores ---
Write-Host "== generando vectores (oraculo Python) =="
& python $vectorPy gen $outDir
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: vector.py gen fallo"; exit 1 }

Push-Location $repo

# --- 3. OFFLINE ---
Write-Host ""
Write-Host "== OFFLINE: parser + builder + answer() =="
$manifest = Get-Content (Join-Path $outDir "vectors_manifest.txt")
$vecFiles = @()
foreach ($m in $manifest) { if ($m.Trim() -ne "") { $vecFiles += $m.Trim() } }
$clxArgs = @("run", "examples\dns\src\main.clsx", "--", "offline") + $vecFiles
& $clx @clxArgs 1> (Join-Path $outDir "cls_offline.txt") 2> (Join-Path $outDir "cls_offline_err.txt")
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: runner CLS offline fallo. stderr:"
    Get-Content (Join-Path $outDir "cls_offline_err.txt") | Select-Object -Last 15
    Pop-Location; exit 1
}
$ok = (Compare-Text (Join-Path $outDir "cls_offline.txt") (Join-Path $outDir "offline_expected.txt") "OFFLINE CHECK (parser + answer round-trip)") -and $ok
$ok = (Compare-Text (Join-Path $outDir "cls_queries.hex") (Join-Path $outDir "queries_expected.txt") "BUILDER queries (CLS vs Python)") -and $ok

# --- 4. LIVE ---
Write-Host ""
Write-Host "== LIVE: servidor autoritativo Python (UDP+TCP) + runner CLS =="
$logPath = Join-Path $outDir "queries.log"
if (Test-Path $logPath) { Remove-Item -LiteralPath $logPath -Force }
$srv = Start-Process -FilePath "python" -ArgumentList @($serverPy, "--port", "15353", "--log", $logPath) -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 1500
try {
    & $clx run "examples\dns\src\main.clsx" -- live 127.0.0.1 15353 1> (Join-Path $outDir "cls_live.txt") 2> (Join-Path $outDir "cls_live_err.txt")
    $liveExit = $LASTEXITCODE
} finally {
    Stop-Process -Id $srv.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 200
}
if ($liveExit -ne 0) {
    Write-Host "ERROR: runner CLS live fallo (exit=$liveExit). stderr:"
    Get-Content (Join-Path $outDir "cls_live_err.txt") | Select-Object -Last 15
    Pop-Location; exit 1
}
$ok = (Compare-Text (Join-Path $outDir "cls_live.txt") (Join-Path $outDir "live_expected.txt") "LIVE CHECK (UDP + TCP retry)") -and $ok

# queries recibidas por el server vs esperadas (estricto: orden + mayusculas)
$got = (([System.IO.File]::ReadAllText($logPath)) -replace "`r", "").TrimEnd("`n")
$exp = (([System.IO.File]::ReadAllText((Join-Path $outDir "live_queries_expected.txt"))) -replace "`r", "").TrimEnd("`n")
$gotN = @($got -split "`n" | Where-Object { $_ -ne "" }).Count
if ($got -ceq $exp) {
    Write-Host ("  [ OK ] queries recibidas por el server: {0} identicas" -f $gotN)
} else {
    Write-Host "  [FAIL] queries recibidas != esperadas (orden/mayusculas)"
    $ok = $false
}

# --- 5. nslookup opcional ---
Write-Host ""
Write-Host "== nslookup opcional =="
$nsl = Get-Command nslookup -ErrorAction SilentlyContinue
if (-not $nsl) {
    Write-Host "  [SKIP] nslookup no disponible"
} else {
    $srv2 = Start-Process -FilePath "python" -ArgumentList @($serverPy, "--port", "15353") -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 1500
    $oldEA = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    try {
        $ns = (& nslookup -port=15353 -type=A example.test 127.0.0.1 2>&1 | Out-String)
    } finally {
        $ErrorActionPreference = $oldEA
        Stop-Process -Id $srv2.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 200
    }
    if ($ns -match "192\.0\.2\.1") {
        Write-Host "  [ OK ] nslookup devolvio 192.0.2.1 (oraculo externo del SO)"
    } else {
        Write-Host "  [SKIP] nslookup no devolvio el A esperado (entorno); salida:"
        ($ns -split "`n" | Select-Object -First 6) | ForEach-Object { Write-Host ("         " + $_) }
    }
}

Pop-Location
Write-Host ""
if ($ok) {
    Write-Host "RESULTADO: OK (codec/builder/transporte CLS == oraculo Python)"
    exit 0
}
Write-Host "RESULTADO: DIVERGENCIA (ver arriba)"
exit 1
