# _runner.ps1 — runner de re-auditoria Fase 2 (v1)
# Uso: powershell -File _runner.ps1 <ruta.clsx> [-HangTimeoutMs N] [-Walk] [-Env KEY=VALUE]
# Captura stdout/stderr a archivos UTF-8, devuelve exit code, mide tiempo y
# detecta cuelgues (matando el proceso si supera el timeout).
param(
    [Parameter(Mandatory=$true)][string]$Script,
    [int]$HangTimeoutMs = 0,
    [switch]$Walk,
    [string]$Env = ""
)
$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logs = "C:\Users\Frask\Documents\cls\examples\audit\fase-2\_logs"
New-Item -ItemType Directory -Force -Path $logs | Out-Null

$script = (Resolve-Path $Script).Path
$base = [System.IO.Path]::GetFileNameWithoutExtension($script)

$outF = Join-Path $env:TEMP ("fase2_{0}_out.txt" -f $base)
$errF = Join-Path $env:TEMP ("fase2_{0}_err.txt" -f $base)
Remove-Item $outF, $errF -ErrorAction SilentlyContinue

$args = @("run", "--jit", $script)
if ($Walk) { $args = @("run", $script) }

$savedEnv = $null
if ($Env -ne "") {
    $parts = $Env -split "="
    $savedEnv = [System.Environment]::GetEnvironmentVariable($parts[0])
    [System.Environment]::SetEnvironmentVariable($parts[0], $parts[1])
}

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$mode = if ($Walk) { "walk" } else { "jit" }
$code = ""
$hung = $false

if ($HangTimeoutMs -gt 0) {
    $p = Start-Process -FilePath $clx -ArgumentList $args -NoNewWindow -PassThru `
        -RedirectStandardOutput $outF -RedirectStandardError $errF
    if (-not $p.WaitForExit($HangTimeoutMs)) {
        $p.Kill(); $p.WaitForExit(); $hung = $true
    }
} else {
    $p = Start-Process -FilePath $clx -ArgumentList $args -Wait -PassThru `
        -RedirectStandardOutput $outF -RedirectStandardError $errF
}
$sw.Stop()
$out = Get-Content -Raw -Encoding UTF8 $outF -ErrorAction SilentlyContinue
$err = Get-Content -Raw -Encoding UTF8 $errF -ErrorAction SilentlyContinue

if ($savedEnv -ne $null) {
    [System.Environment]::SetEnvironmentVariable($parts[0], $savedEnv)
}

if ($hung) { $code = "TIMEOUT" } else { $code = $p.ExitCode }
$suffix = if ($Walk) { ".walker.log" } else { ".log" }
$logFile = Join-Path $logs ("{0}.{1}{2}" -f $base, $mode, $suffix)
@("=== $script ===", "MODE=$mode", "EXITCODE=$code", "TIME_MS=$($sw.ElapsedMilliseconds)", "--- STDOUT ---", $out, "--- STDERR ---", $err, "================") |
    Set-Content $logFile -Encoding UTF8

Write-Host ("=== {0} ({1}) ===" -f $base, $mode)
Write-Host "EXITCODE=$code  TIME_MS=$($sw.ElapsedMilliseconds)"
Write-Host "--- STDOUT ---"
Write-Host $out
Write-Host "--- STDERR ---"
Write-Host $err
Write-Host "LOG=$logFile"
