# run.ps1 - Runner diferencial de la prueba Shell POSIX (clsh) contra dash.
#
#   * corre cada tests/*.sh con clsh (CLS -> WASM -> wasmtime, JIT) y con dash
#   * normaliza CRLF -> LF en ambos lados
#   * compara stdout, stderr y exit code BYTE A BYTE
#   * regenera tests/<name>.expect (stdout dash), .expect.err y .expect.exit
#   * falla (exit != 0) si CUALQUIER test diverge
#
# Uso:  powershell -File examples\clsh\run.ps1
#
# Requiere: target\debug\clx.exe compilado y dash en la ruta conocida (abajo).
# CWD = raiz del repo (el .dll de FFI se resuelve relativo al CWD).

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..")).Path
$clx = Join-Path $repo "target\debug\clx.exe"
$main = "examples\clsh\src\main.clsx"
$testsDir = Join-Path $here "tests"

# Oraculo externo: dash (MSYS2 devkitPro). Alternativas: Git bash --posix.
$dash = "c:\devkitPro\msys2\usr\bin\dash.exe"
if (-not (Test-Path $dash)) {
    $alt = "C:\Program Files\Git\usr\bin\sh.exe"
    if (Test-Path $alt) { $dash = $alt } else {
        Write-Host "ERROR: no se encontro dash ni sh (oraculo externo)."
        exit 2
    }
}
if (-not (Test-Path $clx)) {
    Write-Host "ERROR: no existe $clx. Compila con: cargo build --bin clx"
    exit 2
}

# --- modulo compartido de I/O (modules/io): unico acceso al SO via FFI ---
# (extraido de la prueba a /modules; ver examples/patrones.md §11.5)
$buildPs1 = Join-Path $repo "modules\io\build.ps1"
if (-not (Test-Path $buildPs1)) { Write-Host "ERROR: no existe $buildPs1"; exit 2 }
& $buildPs1
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: modules/io/build.ps1 fallo."; exit 1 }

# Limite de tiempo para que un bucle infinito no cuelgue la suite.
$env:CLS_TIME_LIMIT = "20000"
$env:CLS_JIT_TRACE = "0"   # sin shadow stack: menos codigo WASM, determinismo

function QuoteArg([string]$a) {
    if ($a -match '[\s"]') { return '"' + ($a -replace '"', '\"') + '"' }
    return $a
}

# Captura binaria (stdout/stderr crudos) + exit code, sin pasar por texto.
function Capture {
    param([string]$Exe, [string[]]$Argv)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.WorkingDirectory = $repo
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = (($Argv | ForEach-Object { QuoteArg $_ }) -join ' ')
    $p = [System.Diagnostics.Process]::Start($psi)
    $msOut = New-Object System.IO.MemoryStream
    $msErr = New-Object System.IO.MemoryStream
    $p.StandardOutput.BaseStream.CopyTo($msOut)
    $p.StandardError.BaseStream.CopyTo($msErr)
    if (-not $p.WaitForExit(60000)) { $p.Kill(); return @{ Out = @(); Err = @(); Code = -999 } }
    return @{ Out = $msOut.ToArray(); Err = $msErr.ToArray(); Code = $p.ExitCode }
}

# Normaliza CRLF -> LF (dash en MSYS emite \n; clsh en binario tambien, pero
# puede quedar algun CR segun el SO).
function NormCrlf {
    param([byte[]]$B)
    $o = New-Object 'System.Collections.Generic.List[byte]'
    for ($i = 0; $i -lt $B.Length; $i++) {
        if ($B[$i] -eq 13 -and ($i + 1) -lt $B.Length -and $B[$i + 1] -eq 10) { continue }
        $o.Add($B[$i])
    }
    return ,$o.ToArray()
}

function BytesEq {
    param([byte[]]$A, [byte[]]$B)
    if ($A.Length -ne $B.Length) { return $false }
    for ($i = 0; $i -lt $A.Length; $i++) { if ($A[$i] -ne $B[$i]) { return $false } }
    return $true
}

function ShowBytes {
    param([string]$Label, [byte[]]$B)
    $s = [System.Text.Encoding]::UTF8.GetString($B)
    Write-Host "    $Label ($($B.Length) B): $($s -replace "`n", '\n')"
}

$tests = Get-ChildItem -Path $testsDir -Filter *.sh | Sort-Object Name
if ($tests.Count -eq 0) { Write-Host "ERROR: no hay tests/*.sh"; exit 2 }

$fail = 0
$pass = 0
Write-Host "=== clsh vs dash (diferencial byte a byte) ==="
Write-Host "oraculo: $dash"
Write-Host ""

foreach ($t in $tests) {
    $name = $t.BaseName
    $script = $t.FullName

    $rC = Capture $clx @("run", $main, "--", $script)
    $rD = Capture $dash @($script)

    $oc = NormCrlf $rC.Out; $ec = NormCrlf $rC.Err; $cc = $rC.Code
    $od = NormCrlf $rD.Out; $ed = NormCrlf $rD.Err; $cd = $rD.Code

    # persistir expectativas (oraculo dash)
    [System.IO.File]::WriteAllBytes((Join-Path $testsDir "$name.expect"), $od)
    [System.IO.File]::WriteAllBytes((Join-Path $testsDir "$name.expect.err"), $ed)
    Set-Content -Path (Join-Path $testsDir "$name.expect.exit") -Value $cd -NoNewline

    $ok = (BytesEq $oc $od) -and (BytesEq $ec $ed) -and ($cc -eq $cd)
    if ($ok) {
        $pass++
        Write-Host ("  [ OK ] {0,-12} exit={1} out={2}B" -f $name, $cd, $od.Length)
    } else {
        $fail++
        Write-Host ("  [FAIL] {0,-12} exit clsh={1} dash={2}" -f $name, $cc, $cd)
        if (-not (BytesEq $oc $od)) {
            Write-Host "    stdout DIFF:"
            ShowBytes "clsh" $oc
            ShowBytes "dash" $od
        }
        if (-not (BytesEq $ec $ed)) {
            Write-Host "    stderr DIFF:"
            ShowBytes "clsh" $ec
            ShowBytes "dash" $ed
        }
    }
}

# limpieza de artefactos de los tests
Get-ChildItem -Path $repo -Filter "clsh_t_*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
    Write-Host "RESULTADO: OK ($pass/$($tests.Count) tests byte-identicos a dash)"
    exit 0
}
Write-Host "RESULTADO: $fail/$($tests.Count) test(s) DIVERGEN de dash"
exit 1
