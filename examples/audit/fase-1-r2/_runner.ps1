# Runner destructivo Fase 1 r2 - ejecuta clx run --jit sobre scripts de auditoria
# Job externo da timeout real; Start-Process -Wait -PassThru da exit code fiable.
param(
    [string]$Script,
    [int]$TimeoutMs = 10000
)

$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logs = "C:\Users\Frask\Documents\cls\examples\audit\fase-1-r2\_logs"
if (-not (Test-Path $logs)) { New-Item -ItemType Directory -Force -Path $logs | Out-Null }
$name = [System.IO.Path]::GetFileNameWithoutExtension($Script)
$out = Join-Path $logs "$name.jit.log"
$err = Join-Path $logs "$name.jit.err.log"
if (Test-Path $out) { Remove-Item $out }
if (Test-Path $err) { Remove-Item $err }

$job = Start-Job -ScriptBlock {
    param($clx, $script, $out, $err)
    $p = Start-Process -FilePath $clx -ArgumentList "run --jit", $script -PassThru -Wait -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err
    $p.ExitCode
} -ArgumentList $clx, $Script, $out, $err

$waitSecs = [int](($TimeoutMs + 30000) / 1000)
if (-not (Wait-Job $job -Timeout $waitSecs)) {
    Stop-Job $job -ErrorAction SilentlyContinue
    Remove-Job $job -Force -ErrorAction SilentlyContinue
    "STATUS: TIMEOUT/INFINITE"
    "EXIT: 137"
} else {
    $code = Receive-Job $job
    Remove-Job $job -Force
    "STATUS: EXITED"
    "EXIT: $code"
}
"---- STDOUT ----"
if (Test-Path $out) { Get-Content $out }
"---- STDERR ----"
if (Test-Path $err) { Get-Content $err }
