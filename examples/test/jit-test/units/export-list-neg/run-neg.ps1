# run-neg.ps1 - Valida que los `export { ... }` invalidos fallen el typeck.
# El runner principal (run-tests.ps1) solo globa units/*.clsx (no recursivo),
# asi que estos fixtures negativos viven en este subdirectorio y se corren aparte.
#
# Uso: powershell -File run-neg.ps1
[CmdletBinding()]
param()
$ErrorActionPreference = "Continue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..\..\..")).Path
$clx = (Resolve-Path (Join-Path $root "target\debug\clx.exe")).Path

$pass = 0; $fail = 0
Write-Host "== export { ... } - validaciones negativas =="
Get-ChildItem $PSScriptRoot -Filter *.clsx | Sort-Object Name | ForEach-Object {
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    $p = Start-Process -FilePath $clx -ArgumentList @("run", $_.FullName) -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $err = (Get-Content -Raw -Encoding UTF8 $tmpErr -ErrorAction SilentlyContinue) + ""
    Remove-Item $tmpOut, $tmpErr -ErrorAction SilentlyContinue
    if ($p.ExitCode -ne 0 -and -not [string]::IsNullOrWhiteSpace($err)) {
        $pass++
        $line = ($err -split "`r?`n" | Where-Object { $_ -match "ERROR|Error" } | Select-Object -First 1)
        Write-Host ("{0,-24} PASS {1}" -f $_.Name, $line)
    } else {
        $fail++
        Write-Host ("{0,-24} FAIL (esperaba error de typeck)" -f $_.Name)
    }
}
Write-Host "Resultado: $pass PASS, $fail FAIL"
if ($fail -gt 0) { exit 1 }
