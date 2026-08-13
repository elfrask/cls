# _release-f-dx.ps1 - SECCION F: experiencia de release (flujo usuario nuevo)
. "C:\Users\Frask\Documents\cls\examples\audit\release\_run-lib.ps1"

function Write-U8($path, $text) {
    [System.IO.File]::WriteAllText($path, $text, [System.Text.UTF8Encoding]::new($false))
}

"===== F: DX FLUJO USUARIO NUEVO ====="

$dxDir = Join-Path $audit "release\_dx"
Remove-Item $dxDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $dxDir | Out-Null

# F1: clx new crea el proyecto
$code = Run-Raw "dx-new" @("new", "miapp") 30000 $dxDir
$newOut = Get-Content (Join-Path $logDir "dx-new.out.txt") -Raw
"F1 clx new: exit=$code out=$newOut"
$created = Get-ChildItem $dxDir -Recurse -File | ForEach-Object { $_.FullName.Replace($dxDir, "") }
$created
Add-Result "dx-new" $(if ($code -eq 0 -and (Test-Path (Join-Path $dxDir "miapp\src\main.clsx")) -and (Test-Path (Join-Path $dxDir "miapp\cls.json"))) { "PASS" } else { "FAIL" })

$proj = Join-Path $dxDir "miapp"
$mainFile = Join-Path $proj "src\main.clsx"

# F2: correr el main generado
$code = Run-Jit "dx-run-generado" $mainFile
$out2 = Get-Content (Join-Path $logDir "dx-run-generado.out.txt") -Raw
"F2 run main generado: exit=$code out=$out2"
Add-Result "dx-run-generado" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

# F3: el usuario agrega un modulo y lo importa
Write-U8 (Join-Path $proj "src\util.clsx") @"
export function duplicar(n: int) -> int {
    return n * 2;
};
"@
Write-U8 $mainFile @"
import "util" as u;
function main(args: String[]) -> int {
    print("duplicar(21):", u::duplicar(21));
    return 0;
};
"@
$code = Run-Jit "dx-run-modulo" $mainFile
$out3 = Get-Content (Join-Path $logDir "dx-run-modulo.out.txt") -Raw
"F3 run con modulo importado: exit=$code out=$out3"
Add-Result "dx-run-modulo" $(if ($code -eq 0 -and $out3 -match "42") { "PASS" } else { "FAIL" })

# F4: error dentro del modulo -> trace completo (import_trace + call stack + caret), sin Trap WASM
Write-U8 (Join-Path $proj "src\util.clsx") @"
export function romper() -> int {
    var x = 10;
    return x / 0;
};
"@
Write-U8 $mainFile @"
import "util" as u;
function main(args: String[]) -> int {
    print("llamando...");
    var r = u::romper();
    print("r:", r);
    return 0;
};
"@
$code = Run-Jit "dx-error-modulo" $mainFile
$err4 = Get-Content (Join-Path $logDir "dx-error-modulo.err.txt") -Raw
$hasTrap = $err4 -match "Trap WASM"
$hasTrace = $err4 -match "→"
$hasCaret = $err4 -match "\^"
$hasArchivo = $err4 -match "util.clsx"
$hasError = $err4 -match "Error"
"F4 error en modulo: exit=$code trap=$hasTrap trace=$hasTrace caret=$hasCaret archivo=$hasArchivo err=$hasError"
Add-Result "dx-error-modulo" $(if ($code -ne 0 -and -not $hasTrap -and $hasTrace -and $hasCaret -and $hasArchivo) { "PASS" } else { "FAIL" })

# F5: examples/hello corre bien (proyecto canónico)
$code = Run-Jit "dx-hello" "C:\Users\Frask\Documents\cls\examples\hello\src\main.clsx"
$out5 = Get-Content (Join-Path $logDir "dx-hello.out.txt") -Raw
"F5 examples/hello: exit=$code out=$out5"
Add-Result "dx-hello" $(if ($code -eq 0) { "PASS" } else { "FAIL" })

"===== RESUMEN F ====="
$script:results | Format-Table -AutoSize | Out-String | Write-Output
