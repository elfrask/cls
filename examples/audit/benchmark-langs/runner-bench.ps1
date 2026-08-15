# runner-bench.ps1 - Ejecuta benchmarks de CLS/C++/Rust/Python/JS y genera CSV + tabla.
$ErrorActionPreference = "Continue"
$root = "C:\Users\Frask\Documents\cls"
$dir = Join-Path $root "examples\audit\benchmark-langs"
$clx = Join-Path $root "target\debug\clx.exe"

Remove-Item "$env:USERPROFILE\.cache\cls\*.wasm" -Force -ErrorAction SilentlyContinue

$results = @()   # (lang, bench, internal_ms, wall_ms)

function Measure-Bench($lang, $bench, $scriptBlock) {
    for ($k = 1; $k -le 3; $k++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $out = (& $scriptBlock 2>&1 | Out-String)
        $sw.Stop()
        $wall = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        $internal = $null
        foreach ($line in ($out -split "`r?`n")) {
            if ($line -match "^" + [regex]::Escape("$bench`_ms") + ": ([\d\.]+)") {
                $internal = [math]::Round([double]$matches[1], 3)
            }
        }
        $script:results += [PSCustomObject]@{
            Lang = $lang; Bench = $bench; Run = $k
            InternalMs = $internal; WallMs = $wall
        }
        if ($k -eq 1) { Write-Host ("{0,-8} {1,-8} run{2}: internal={3}ms wall={4}ms" -f $lang, $bench, $k, $internal, $wall) }
    }
}

Write-Host "== Benchmarks =="
# CLS: 6 pruebas por separado (el nombre del bench = prefijo del output *_ms:)
$clsTests = @(
    @{ File = "01-arith";  Bench = "arith" },
    @{ File = "02-fib";    Bench = "fib" },
    @{ File = "03-array";  Bench = "arr" },
    @{ File = "04-string"; Bench = "str" },
    @{ File = "05-math";   Bench = "math" },
    @{ File = "06-calls";  Bench = "call" }
)
foreach ($t in $clsTests) {
    Measure-Bench "cls" $t.Bench { & $clx run (Join-Path $dir "cls\$($t.File).clsx") }
}
# C++ / Rust / Python / JS: cada uno ejecuta las 6 en un solo proceso
Measure-Bench "cpp"   "arith" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "arith" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "arith" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "arith" { & node (Join-Path $dir "js\bench.js") }
Measure-Bench "cpp"   "fib" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "fib" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "fib" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "fib" { & node (Join-Path $dir "js\bench.js") }
Measure-Bench "cpp"   "arr" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "arr" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "arr" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "arr" { & node (Join-Path $dir "js\bench.js") }
Measure-Bench "cpp"   "str" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "str" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "str" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "str" { & node (Join-Path $dir "js\bench.js") }
Measure-Bench "cpp"   "math" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "math" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "math" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "math" { & node (Join-Path $dir "js\bench.js") }
Measure-Bench "cpp"   "call" { & (Join-Path $dir "cpp\bench.exe") }
Measure-Bench "rust"  "call" { & (Join-Path $dir "rust\target\release\bench.exe") }
Measure-Bench "python" "call" { & python (Join-Path $dir "python\bench.py") }
Measure-Bench "js"    "call" { & node (Join-Path $dir "js\bench.js") }

$csv = Join-Path $dir "resultados.csv"
$results | Export-Csv -Path $csv -NoTypeInformation -Encoding UTF8
Write-Host ""
Write-Host "CSV guardado en: $csv"

# Tabla resumen: promedio de internal_ms por (lang, bench)
Write-Host ""
Write-Host "== Resumen (internal ms, promedio de 3 runs) =="
$results | Where-Object { $_.InternalMs -ne $null } |
    Group-Object Lang, Bench |
    ForEach-Object {
        $avg = [math]::Round(($_.Group | Measure-Object InternalMs -Average).Average, 3)
        $parts = $_.Name -split ", "
        [PSCustomObject]@{ Lang = $parts[0]; Bench = $parts[1]; AvgInternalMs = $avg }
    } |
    Sort-Object Bench, Lang |
    Format-Table -AutoSize
