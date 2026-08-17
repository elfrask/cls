# comparar-paridad.ps1 - Ejecuta scripts con master y refactor, compara salidas exactas.
$ErrorActionPreference = "Continue"
$master = "C:\Users\Frask\AppData\Local\Temp\opencode\cls-master-target\debug\clx.exe"
$refact = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$root = "C:\Users\Frask\Documents\cls"

$scripts = @(
    "examples/audit/features/01-basics.clsx",
    "examples/audit/features/02-operadores.clsx",
    "examples/audit/features/04-arrays.clsx",
    "examples/audit/features/05-tuplas.clsx",
    "examples/audit/features/06-records.clsx",
    "examples/audit/features/07-control-flujo.clsx",
    "examples/audit/features/08-funciones.clsx",
    "examples/audit/features/09-clases.clsx",
    "examples/audit/features/10-enums.clsx",
    "examples/audit/features/11-structs.clsx",
    "examples/audit/features/12-cmx.clsx",
    "examples/audit/features/13-stdlib.clsx",
    "examples/audit/features/14-intrinsics.clsx",
    "examples/audit/features/15-try-catch.clsx",
    "examples/audit/features/16-magic-methods.clsx",
    "examples/audit/features/17-genericos.clsx",
    "examples/audit/features/18-shapes.clsx",
    "examples/audit/test-features/tests/jit-magic-all.clsx"
)

$pass = 0; $fail = 0; $fails = @()
foreach ($s in $scripts) {
    $full = Join-Path $root $s
    $m = (& $master run $full 2>&1 | Out-String)
    $r = (& $refact run $full 2>&1 | Out-String)
    $mCode = $LASTEXITCODE
    if ($m.Trim() -eq $r.Trim()) { $pass++; "$s : IDENTICO" }
    else { $fail++; $fails += $s; "$s : DIFF" }
}
Write-Host ""
Write-Host "Paridad: $pass PASS, $fail FAIL"
if ($fails) { $fails | ForEach-Object { Write-Host "  FALLA: $_" } }
