# comparar-errores.ps1 - Compara formato de errores entre master y refactor.
$ErrorActionPreference = "Continue"
$master = "C:\Users\Frask\AppData\Local\Temp\opencode\cls-master-target\debug\clx.exe"
$refact = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$root = "C:\Users\Frask\Documents\cls"

$scripts = @(
    "examples/audit/errores/err-divzero-nested.clsx",
    "examples/audit/errores/err-throw-no-capturado.clsx",
    "examples/audit/errores/err-index-fuera.clsx",
    "examples/audit/errores/err-miembro-inexistente.clsx",
    "examples/audit/errores/err-type-mismatch.clsx",
    "examples/audit/modules/err-mod-no-existe.clsx",
    "examples/audit/modules/err-error-en-modulo.clsx",
    "examples/audit/stress/stress-recursion.clsx"
)
$pass = 0; $fail = 0
foreach ($s in $scripts) {
    $full = Join-Path $root $s
    $m = (& $master run $full 2>&1 | Out-String)
    $r = (& $refact run $full 2>&1 | Out-String)
    # normalizar: quitar solo el timestamp si existe
    if ($m.Trim() -eq $r.Trim()) { $pass++; "$s : IDENTICO" }
    else { $fail++; "$s : DIFF" }
}
"Paridad errores: $pass PASS, $fail FAIL"
