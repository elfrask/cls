# _release-e-cli.ps1 - SECCION E: CLI (exit codes correctos)
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

"===== E: CLI ====="

# E1 --version
$code = Run-Raw "cli-version" @("--version")
$v = Get-Content (Join-Path $logDir "cli-version.out.txt") -Raw
"E1 --version: exit=$code out=$v"
Add-Result "cli-version" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E2 -h
$code = Run-Raw "cli-help" @("-h")
"E2 -h: exit=$code"
Add-Result "cli-help" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E3 run sin archivo
$code = Run-Raw "cli-run-noarg" @("run")
$e3 = Get-Content (Join-Path $logDir "cli-run-noarg.err.txt") -Raw
"E3 run sin archivo: exit=$code err=$e3"
Add-Result "cli-run-noarg" $(if ($code -ne 0) { "PASS" } else { "FAIL" })

# E4 check sobre archivo valido
$code = Run-Raw "cli-check" @("check", "C:\Users\Frask\Documents\cls\examples\audit\release\x1-pow.clsx")
"E4 check: exit=$code"
Add-Result "cli-check" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E5 ast
$code = Run-Raw "cli-ast" @("ast", "C:\Users\Frask\Documents\cls\examples\audit\release\x1-pow.clsx")
"E5 ast: exit=$code"
Add-Result "cli-ast" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E6 clean
$code = Run-Raw "cli-clean" @("clean")
"E6 clean: exit=$code"
Add-Result "cli-clean" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E7 repl con exit
$stdin = Join-Path $audit "release\_logs\repl-stdin.txt"
Set-Content $stdin "exit" -NoNewline
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $clx
$psi.Arguments = "repl"
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.RedirectStandardInput = $true
$psi.CreateNoWindow = $true
$pr = New-Object System.Diagnostics.Process
$pr.StartInfo = $psi
$null = $pr.Start()
$pr.StandardInput.WriteLine("exit")
$pr.StandardInput.Close()
$outTask = $pr.StandardOutput.ReadToEndAsync()
$errTask = $pr.StandardError.ReadToEndAsync()
if (-not $pr.WaitForExit(10000)) { try { $pr.Kill() } catch {}; "E7 repl: TIMEOUT"; Add-Result "cli-repl" "FAIL" }
else {
    [System.IO.File]::WriteAllText((Join-Path $logDir "cli-repl.out.txt"), $outTask.Result)
    [System.IO.File]::WriteAllText((Join-Path $logDir "cli-repl.err.txt"), $errTask.Result)
    "E7 repl: exit=$($pr.ExitCode)"
    Add-Result "cli-repl" $(if ($pr.ExitCode -eq 0) { "PASS" } else { "FAIL" })
}

# E8 tree (placeholder) en subcarpeta aislada
$cliDir = Join-Path $audit "release\_cli"
New-Item -ItemType Directory -Force -Path $cliDir | Out-Null
$code = Run-Raw "cli-tree" @("tree") 30000 $cliDir
"E8 tree: exit=$code"
Add-Result "cli-tree" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E9 fmt (placeholder) en subcarpeta aislada
$code = Run-Raw "cli-fmt" @("fmt") 30000 $cliDir
"E9 fmt: exit=$code"
Add-Result "cli-fmt" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# E10 init (placeholder) en subcarpeta aislada
$code = Run-Raw "cli-init" @("init") 30000 $cliDir
"E10 init: exit=$code"
Add-Result "cli-init" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

"===== RESUMEN E ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
