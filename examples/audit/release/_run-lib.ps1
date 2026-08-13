# _run-lib.ps1 - helpers compartidos (se invoca con dot-source)
$ErrorActionPreference = "Continue"
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$audit = "C:\Users\Frask\Documents\cls\examples\audit"
$logDir = Join-Path $audit "release\_logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

$script:results = @()
$script:badStrings = @("Módulo WASM inválido", "Trap WASM", "index_out_of_bounds", "unreachable")

function New-Proc($argsLine, [string]$workDir = "") {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $clx
    $psi.Arguments = $argsLine
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    if ($workDir -ne "") { $psi.WorkingDirectory = $workDir }
    $p = New-Object System.Diagnostics.Process
    $p.StartInfo = $psi
    return $p
}

function Run-Proc($label, $argsLine, [int]$timeoutMs = 30000, [string]$workDir = "") {
    $outFile = Join-Path $logDir "$label.out.txt"
    $errFile = Join-Path $logDir "$label.err.txt"
    $p = New-Proc $argsLine $workDir
    $null = $p.Start()
    $outTask = $p.StandardOutput.ReadToEndAsync()
    $errTask = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit($timeoutMs)) {
        try { $p.Kill() } catch {}
        $null = $p.WaitForExit(5000)
        return "TIMEOUT"
    }
    [System.IO.File]::WriteAllText($outFile, $outTask.Result)
    [System.IO.File]::WriteAllText($errFile, $errTask.Result)
    return $p.ExitCode
}

function Run-Jit($label, $file, [int]$timeoutMs = 30000) {
    return Run-Proc $label ("run --jit `"{0}`"" -f $file) $timeoutMs
}

function Run-Raw($label, $argsList, [int]$timeoutMs = 30000, [string]$workDir = "") {
    return Run-Proc $label (($argsList | ForEach-Object { "`"{0}`"" -f $_ }) -join " ") $timeoutMs $workDir
}

function Check-BadStrings($label) {
    $outFile = Join-Path $logDir "$label.out.txt"
    $errFile = Join-Path $logDir "$label.err.txt"
    $content = ""
    if (Test-Path $outFile) { $content += Get-Content $outFile -Raw }
    if (Test-Path $errFile) { $content += Get-Content $errFile -Raw }
    foreach ($b in $script:badStrings) {
        if ($content -match [regex]::Escape($b)) {
            return $b
        }
    }
    return $null
}

function Add-Result($label, $status) {
    $script:results += [PSCustomObject]@{ Label = $label; Status = $status }
}
