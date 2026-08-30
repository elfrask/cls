# run-audit.ps1 - Runner de auditoria QA practico (v4, JIT-only).
# Migracion dev-2 (Fase 7): el walker fue eliminado del repo. Este
# script ya no puede comparar walker vs JIT. Solo mide JIT.
#
# Uso: powershell -File run-audit.ps1 <ruta.clsx>
param(
    [Parameter(Mandatory=$true)][string]$Script
)
$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logs = "C:\Users\Frask\Documents\cls\examples\audit\_logs"
New-Item -ItemType Directory -Force -Path $logs | Out-Null

$script = (Resolve-Path $Script).Path
$base = [System.IO.Path]::GetFileNameWithoutExtension($script)

function Run-Jit($f) {
    $outF = Join-Path $env:TEMP ("qa_audit_jit_out.txt")
    $errF = Join-Path $env:TEMP ("qa_audit_jit_err.txt")
    Remove-Item $outF, $errF -ErrorAction SilentlyContinue
    $args = @("run", $f)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $clx -ArgumentList $args -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $outF -RedirectStandardError $errF
    $sw.Stop()
    $out = Get-Content -Raw -Encoding UTF8 $outF -ErrorAction SilentlyContinue
    $err = Get-Content -Raw -Encoding UTF8 $errF -ErrorAction SilentlyContinue
    return [pscustomobject]@{ Out = $out; Err = $err; Code = $p.ExitCode; Ms = $sw.ElapsedMilliseconds }
}

$jit = Run-Jit $script
$logJit = Join-Path $logs "$base.jit.log"
@("=== $script ===", "EXITCODE=$($jit.Code)", "TIME_MS=$($jit.Ms)", "--- STDOUT ---", $jit.Out, "--- STDERR ---", $jit.Err, "================") | Set-Content $logJit -Encoding UTF8

Write-Host ("{0,-28} JIT={1} {2}ms" -f $base, $jit.Code, $jit.Ms)

if ($jit.Err) {
    $firstErr = ($jit.Err -split "`r?`n" | Where-Object { $_.Trim() -ne "" } | Select-Object -First 2) -join " || "
    Write-Host "    JIT ERR: $firstErr"
}
