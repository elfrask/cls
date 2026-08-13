param(
    [Parameter(Mandatory=$true)][string]$Name,
    [Parameter(Mandatory=$true)][string]$ArgsStr,
    [int]$TimeoutMs = 20000,
    [switch]$StdinExit
)
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$logDir = "C:\Users\Frask\Documents\cls\examples\audit\migracion\_logs"
$stdout = Join-Path $logDir "$Name.out"
$stderr = Join-Path $logDir "$Name.err"
$exitfile = Join-Path $logDir "$Name.exit"

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $clx
$psi.Arguments = $ArgsStr
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
if ($StdinExit) { $psi.RedirectStandardInput = $true }
$psi.CreateNoWindow = $true
$p = New-Object System.Diagnostics.Process
$p.StartInfo = $psi
$null = $p.Start()

if ($StdinExit) {
    Start-Sleep -Milliseconds 800
    try { $p.StandardInput.WriteLine("exit"); $p.StandardInput.Close() } catch {}
}

$timedOut = -not $p.WaitForExit($TimeoutMs)
if ($timedOut) { try { $p.Kill() } catch {} }

$out = $p.StandardOutput.ReadToEnd()
$err = $p.StandardError.ReadToEnd()
[System.IO.File]::WriteAllText($stdout, $out)
[System.IO.File]::WriteAllText($stderr, $err)
$code = $p.ExitCode
[System.IO.File]::WriteAllText($exitfile, "$code")
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Write-Output "=== $Name : exit=$code timedout=$timedOut ==="
Write-Output "--- STDOUT ---"
Write-Output $out
Write-Output "--- STDERR ---"
Write-Output $err
