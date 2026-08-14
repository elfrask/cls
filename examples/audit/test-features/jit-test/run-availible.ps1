# run-availible.ps1 — Prueba las features disponibles del JIT (carpeta availible/).
# Ejecuta cada script con el JIT y el walker, compara paridad y muestra la salida.
# Uso: powershell -File run-availible.ps1
$ErrorActionPreference = "SilentlyContinue"
$clx = Join-Path $PSScriptRoot "..\..\..\target\debug\clx.exe"
$clx = (Resolve-Path $clx).Path
$dir = Join-Path $PSScriptRoot "availible"

$pass = 0; $fail = 0
Write-Host "== Features disponibles del JIT (paridad JIT == walker) =="
Write-Host ("{0,-26} {1,-6} {2}" -f "SCRIPT", "RES", "SALIDA (JIT)")
Write-Host ("{0,-26} {1,-6} {2}" -f "------", "---", "-----")
foreach ($f in Get-ChildItem $dir -Filter *.clsx | Sort-Object Name) {
    $jit = (& $clx run --jit $f.FullName 2>$null | Out-String)
    $walk = (& $clx run --ast-walker $f.FullName 2>$null | Out-String)
    if ($jit -eq $walk) {
        $res = "PASS"
        $pass++
    } else {
        $res = "FAIL"
        $fail++
    }
    $primera = (($jit -split "`r?`n" | Where-Object { $_ -ne "" }) | Select-Object -First 1)
    if (-not $primera) { $primera = "(sin salida)" }
    Write-Host ("{0,-26} {1,-6} {2}" -f $f.Name, $res, $primera)
}
Write-Host ""
Write-Host "Resultado: $pass PASS, $fail FAIL"
