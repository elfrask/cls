# analisis-bench.ps1 - Lee resultados.csv con culture invariante y genera la tabla final.
$ErrorActionPreference = "Stop"
$dir = "C:\Users\Frask\Documents\cls\examples\audit\benchmark-langs"
$ci = [System.Globalization.CultureInfo]::InvariantCulture

$rows = @()
Import-Csv (Join-Path $dir "resultados.csv") | ForEach-Object {
    $rows += [PSCustomObject]@{
        Lang = $_.Lang
        Bench = $_.Bench
        Run = [int]$_.Run
        InternalMs = [double]($_.InternalMs -replace ',', '.')
        WallMs = [double]($_.WallMs -replace ',', '.')
    }
}

Write-Host "== TABLA: tiempo interno promedio (ms) por lenguaje y prueba =="
$order = @("arith","fib","arr","str","math","call")
foreach ($b in $order) {
    Write-Host ""
    Write-Host ("### {0}" -f $b)
    $rows | Where-Object { $_.Bench -eq $b -and $_.InternalMs -gt 0 } |
        Group-Object Lang |
        ForEach-Object {
            $avg = ($_.Group | Measure-Object InternalMs -Average).Average
            [PSCustomObject]@{
                Lang = $_.Name
                AvgMs = [math]::Round($avg, 3)
                WallAvg = [math]::Round(($_.Group | Measure-Object WallMs -Average).Average, 1)
            }
        } |
        Sort-Object AvgMs |
        Format-Table -AutoSize | Out-String | Write-Host
}

Write-Host "== Factor vs el mas rapido (internal ms, mas bajo = mejor) =="
foreach ($b in $order) {
    $group = $rows | Where-Object { $_.Bench -eq $b -and $_.InternalMs -gt 0 } | Group-Object Lang |
        ForEach-Object { [PSCustomObject]@{ Lang = $_.Name; Avg = ($_.Group | Measure-Object InternalMs -Average).Average } }
    $best = ($group | Measure-Object Avg -Minimum).Minimum
    $line = "  {0,-6}: " -f $b
    $line += (($group | Sort-Object Avg | ForEach-Object { "{0}={1:0.###}ms (x{2:0.0})" -f $_.Lang, $_.Avg, ($_.Avg / $best) }) -join "  ")
    Write-Host $line
}
