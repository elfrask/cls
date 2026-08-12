# CLI tests
$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$out = "$env:TEMP\cli_out.txt"
$err = "$env:TEMP\cli_err.txt"
$log = "C:\Users\Frask\Documents\cls\examples\audit\_logs\cli-tests.log"

function T([string]$name, [string[]]$argv) {
    Remove-Item $out, $err -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $clx -ArgumentList $argv -NoNewWindow -Wait -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
    $o = [System.IO.File]::ReadAllText($out) -as [string]
    $e = [System.IO.File]::ReadAllText($err) -as [string]
    $block = "=== $name EXIT=$($p.ExitCode) ==="
    if ($o) { $block += "`nOUT:`n$o" }
    if ($e) { $block += "`nERR:`n$e" }
    Add-Content $log $block
    Write-Host ("{0,-35} EXIT={1}" -f $name, $p.ExitCode)
}

Set-Content $log "=== CLI tests ==="
T "--version" @("--version")
T "-v" @("-v")
T "subcomando inexistente" @("frobnicate")
T "run sin archivo" @("run")
T "run archivo inexistente" @("run", "C:\no\existe.clsx")
T "check sin archivo" @("check")
T "ast sin archivo" @("ast")
T "jit archivo inexistente" @("run", "--jit", "C:\no\existe.clsx")
T "help" @("--help")
T "run --jit archivo ok" @("run", "--jit", "C:\Users\Frask\Documents\cls\examples\audit\features\01-basics.clsx")
