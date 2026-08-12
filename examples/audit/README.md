# Índice de scripts de auditoría QA práctica

Carpeta raíz de scripts: `examples/audit/`
Runner: `examples/audit/run-audit.ps1` (ejecuta JIT + walker, guarda logs UTF-8 en `_logs/`)

## features/ — 18 scripts (uno por categoría), todos exit 0 en JIT
| Script | Feature |
|---|---|
| 01-basics.clsx | variables, const, let, literales, tipos, print |
| 02-operadores.clsx | aritmética, comparación, lógica, bitwise, compuestos, postfix |
| 03-strings.clsx | interpolación `$x`/`${}`, métodos String |
| 04-arrays.clsx | arrays mutables y métodos |
| 05-tuplas.clsx | tuplas inmutables |
| 06-records.clsx | records/diccionarios |
| 07-control-flujo.clsx | if/elif/else, while, loop, for, for-each, switch, with |
| 08-funciones.clsx | funciones, closures, arrows, recursión |
| 09-clases.clsx | clases, herencia, super, is, visibilidad |
| 10-enums.clsx | enums |
| 11-structs.clsx | structures |
| 12-cmx.clsx | CMX básico |
| 13-stdlib.clsx | math, json, fs |
| 14-intrinsics.clsx | print, len, type, int/str/float/bool, now |
| 15-try-catch.clsx | try/catch/finally, throw |
| 16-magic-methods.clsx | magic methods (subset JIT) |
| 17-genericos.clsx | alias, type access |
| 18-shapes.clsx | shapes, interfaces |

## errores/ — scripts de error y reproducción de bugs
| Script | Tipo | Resultado esperado | Real (JIT) |
|---|---|---|---|
| err-sintaxis-token.clsx | sintaxis | línea+caret | ✅ |
| err-string-sin-cerrar.clsx | sintaxis | línea+caret | ✅ |
| err-llaves.clsx | sintaxis | línea+caret | ✅ |
| err-type-mismatch.clsx | typeck | un nivel + caret | ✅ |
| err-var-no-declarada.clsx | typeck | un nivel + caret | ✅ |
| err-funcion-inexistente.clsx | typeck | un nivel + caret | ✅ |
| err-me-fuera.clsx / err-super-fuera.clsx | typeck | "Variable no definida: me/super" | ✅ (pero el mensaje correcto sería "fuera de clase") |
| err-divzero-nested.clsx | runtime | trace completo | ❌ frame sin call stack |
| err-index-fuera.clsx | runtime | trace | ❌ frame sin call stack |
| err-miembro-inexistente.clsx | runtime | trace | ❌ mensajito suelto |
| err-throw-no-capturado.clsx | runtime | trace | ❌ "Trap WASM: excepción no capturada" |
| err-conversion.clsx | runtime | error conversión | ✅ (explicado en log) |
| err-const-reasignada.clsx | runtime | error const | ⚠️ solo WARN |
| err-private-externo.clsx | visibilidad | ERROR private | ❌ exit 0, imprime 0 (bug H2) |
| **bug-pow-float.clsx** | WASM | — | ❌ WASM inválido + dump WAT |
| **bug-abs-float.clsx** | WASM | — | ❌ WASM inválido |
| **bug-array-hetero-float.clsx** | WASM | — | ❌ WASM inválido |
| **bug-float-cmp-int.clsx** | WASM | — | ❌ WASM inválido |
| **bug-array-index-write.clsx** | WASM | — | ❌ WASM inválido |
| **err-tupla-inmutable.clsx** | WASM | error tuplas | ❌ WASM inválido |
| **bug-len-string.clsx** | runtime | len=4 | ❌ `171798691844` |
| **bug-array-string-puntero.clsx** | runtime | `[1, "dos", 3]` | ❌ `[1, 68719476739, 3]` |
| **bug-range-puntero.clsx** | runtime | `[0..4]` | ❌ `1048600` |
| **bug-fs-cwd-puntero.clsx** | runtime | cwd | ❌ número gigante |
| **bug-bool-string.clsx** | runtime | false/true | ❌ false/false |
| **bug-unario-neg.clsx** | runtime | -5/-5 | ❌ 5/5 |
| **bug-funcion-anidada.clsx** | runtime | ok | ❌ Trap WASM |
| **bug-visibilidad-private.clsx** | visibilidad | ERROR | ❌ exit 0 |
| **bug-visibilidad-protected.clsx** | visibilidad | ERROR | ❌ exit 0 |
| **bug-visibilidad-readonly.clsx** | visibilidad | ERROR | ❌ exit 0 |

## modules/ — sistema de módulos
| Script | Resultado |
|---|---|
| libmod.clsx / privados.clsx / err-lib-modulo.clsx | librerías de prueba |
| mod-import.clsx | ❌ H13 enum namespaced |
| mod-from-include.clsx | ❌ H14 doble import |
| mod-import-anidado.clsx + nested_a + nested_b | ❌ H15 módulos anidados |
| err-mod-no-existe.clsx / err-mod-simbolo.clsx / err-mod-privado.clsx | ✅ errores de resolución correctos |
| err-error-en-modulo.clsx | ❌ H4 trap sin contexto |
| bug-enum-namespaced.clsx / bug-import-doble.clsx | reproducción H13/H14 |
| cachetest/ | setup para prueba de caché (arriba en 07) |

## cmx/ — CMX y sintaxis malformada
- `cmx-*.clsx`: tags sin cerrar, cierre sin apertura, attrs rotos, expr rota, `<div></div>` (H3 cuelga), `< b`, tags especiales, mixto
- `sint-*.clsx`: `(a)` vs tupla, `() =>`, float mal, `1e300` (H9), for-each raro, doble and, interp rota, return/break fuera, `%%%`, `//`, coma colgante

## stress/ — stress tests
| Script | Resultado |
|---|---|
| stress-infinite-while.clsx | TIMEOUT esperado |
| stress-recursion.clsx | ❌ Trap WASM (H4) |
| stress-fact-100k.clsx | ❌ Trap WASM (H4) |
| stress-array-1m.clsx | ❌ TIMEOUT/Trap (H22) |
| stress-string-100k.clsx | ❌ len: 0 (H23) |
| stress-aritmetica.clsx / stress-bucles-anidados.clsx / stress-var-larga.clsx / stress-prints.clsx | ✅ |
| stress-1e300.clsx | ❌ H9 notación científica |
| stress-modulos.clsx | ⚠️ error de ruta del script (no del JIT) |

## perf/ — benchmarks
`perf-loop` (20M iter), `perf-fib` (fib28), `perf-arrays` (100k), `perf-math` (100k), `perf-string` (10k), `perf-llamadas` (1M).

## Logs
`examples/audit/_logs/` — salidas crudas por script: `*.jit.log`, `*.walker.log`,
`*.raw.log` (reproducciones), `cli-tests.log`, `cmx-div-vacio.hang.log`.
