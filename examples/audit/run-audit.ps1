# run-audit.ps1 — Runner de auditoria QA practico (v3, Start-Process con streams limpios).
# Uso: powershell -File run-audit.ps1 <ruta.clsx> [--jit-only]
param(
    [Parameter(Mandatory=$true)][string]$Script,
    [switch]$JitOnly
)
$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logs = "C:\Users\Frask\Documents\cls\examples\audit\_logs"
New-Item -ItemType Directory -Force -Path $logs | Out-Null

$script = (Resolve-Path $Script).Path
$base = [System.IO.Path]::GetFileNameWithoutExtension($script)

function Run-One($mode, $f) {
    $outF = Join-Path $env:TEMP ("qa_audit_{0}_out.txt" -f $mode)
    $errF = Join-Path $env:TEMP ("qa_audit_{0}_err.txt" -f $mode)
    Remove-Item $outF, $errF -ErrorAction SilentlyContinue
    $args = @("run", "--jit", $f)
    if ($mode -eq "walk") { $args = @("run", "--ast-walker", $f) }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $clx -ArgumentList $args -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $outF -RedirectStandardError $errF
    $sw.Stop()
    $out = Get-Content -Raw -Encoding UTF8 $outF -ErrorAction SilentlyContinue
    $err = Get-Content -Raw -Encoding UTF8 $errF -ErrorAction SilentlyContinue
    return [pscustomobject]@{ Out = $out; Err = $err; Code = $p.ExitCode; Ms = $sw.ElapsedMilliseconds }
}

$jit = Run-One "jit" $script
$logJit = Join-Path $logs "$base.jit.log"
@("=== $script ===", "EXITCODE=$($jit.Code)", "TIME_MS=$($jit.Ms)", "--- STDOUT ---", $jit.Out, "--- STDERR ---", $jit.Err, "================") | Set-Content $logJit -Encoding UTF8

if (-not $JitOnly) {
    $walk = Run-One "walk" $script
    $logWalk = Join-Path $logs "$base.walker.log"
    @("=== $script ===", "EXITCODE=$($walk.Code)", "TIME_MS=$($walk.Ms)", "--- STDOUT ---", $walk.Out, "--- STDERR ---", $walk.Err, "================") | Set-Content $logWalk -Encoding UTF8
    $parity = if ($jit.Out -eq $walk.Out) { "PARITY_OK" } else { "PARITY_DIFF" }
    Write-Host ("{0,-28} JIT={1} walker={2} {3}ms/{4}ms {5}" -f $base, $jit.Code, $walk.Code, $jit.Ms, $walk.Ms, $parity)
} else {
    Write-Host ("{0,-28} JIT={1} {2}ms" -f $base, $jit.Code, $jit.Ms)
}

if ($jit.Err) {
    $firstErr = ($jit.Err -split "`r?`n" | Where-Object { $_.Trim() -ne "" } | Select-Object -First 2) -join " || "
    Write-Host "    JIT ERR: $firstErr"
}
