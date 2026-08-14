# Testing

## Unit tests por crate

Todos los crates llevan tests unitarios inline (`#[cfg(test)]`) y tests de
integración en `tests/`. Se ejecutan con:

```ps
cargo test                 # todo el workspace
cargo test -p clxr         # solo un crate
```

| Crate | Dónde | Tests (contados en el código) |
|---|---|---|
| `cls-core` | `src/frontend/lexer.rs` (8), `src/frontend/parser.rs` (22), `src/middleware/typeck.rs` (21), `src/error/mod.rs` (9) | span/merge/fábricas de error, lexer, parser, typeck |
| `cls-runtime` | `src/stdlib/primitive.rs` (10), `src/interpreter.rs` (22), `src/vfs/resolver.rs` (10), `src/vfs/security.rs` (5), `src/error_report.rs` (6) | dispatch tables de primitivos, control de flujo, VFS, formato de errores |
| `cls-jit` | `tests/` — `wasmi_smoke.rs`, `host_call.rs`, `exports.rs`, `debug_kinds.rs` (8 tests) | **`wasmi_smoke` requiere la feature `wasmi-runtime`** |
| `nodos/clxb` | `tests/embed.rs` (5 tests de integración) | `cargo test -p clxb` |

Los tests de `clxb` (`tests/embed.rs`) cubren, con `ClsEngine`:

- `call_exports_scalares` — exports `int/float/bool/String` (suma, ratio, mayor, startsWith).
- `call_exports_arrays_records` — arrays y records como valores.
- `run_main_y_eval` — `run_main` y `eval`.
- `output_capturado` — captura de `print` vía `OutputSink`.
- `sdk_intrinsics` — SDK de nodo (intrinsics + resolver del host).

```ps
cargo test -p clxb
```

## QA de features (JIT)

### `examples/audit/features/`

18 scripts de QA por feature (`01-basics.clsx` … `18-shapes.clsx`):
variables, operadores, strings, arrays, tuplas, records, control de flujo,
funciones, clases, enums, structs, CMX, stdlib, intrinsics, try/catch,
magic methods, genéricos, shapes.

### `examples/audit/test-features/`

| Ruta | Contenido |
|---|---|
| `test-features/jit-test/availible/` | Features disponibles del JIT (25 scripts, `01-operadores.clsx` … `25-bitops.clsx`) |
| `test-features/jit-test/units/` | Unit tests del JIT (`a1`–`a11`, `b1`–`b9`, `f64arr`, `synerr`, `bench_fib`, ...) + `.wat`/`.js` de referencia |
| `test-features/tests/` | Suite de features (imports, VFS, async, errores, magic, clases, ...) |

Logs de corridas: `examples/audit/_logs/*.jit.log` (cada script de QA escribe
`<nombre>.jit.log` con stdout/stderr/exit/time).

## Scripts PowerShell

| Script | Propósito (del encabezado del archivo) |
|---|---|
| `examples/audit/run-audit.ps1` | Runner de auditoría QA práctico (v3, Start-Process con streams limpios). Uso: `powershell -File run-audit.ps1 <ruta.clsx> [--jit-only]` — corre el script con JIT (y walker si no es `--jit-only`), guarda logs en `_logs/` y compara paridad JIT vs walker. |
| `examples/audit/cli-tests.ps1` | CLI tests — batería de subcomandos (`--version`, `run`, `check`, `ast`, `--help`, ...) con exit codes y logs en `_logs/cli-tests.log`. |
| `examples/audit/test-features/jit-test/run-availible.ps1` | Prueba las features disponibles del JIT (carpeta `availible/`); ejecuta cada script con JIT y walker, compara paridad y muestra la salida. |
| `examples/audit/test-features/jit-test/run-tests.ps1` | Prueba cada feature del JIT comparando salidas JIT vs walker (paridad), sobre `units/`; algunos scripts se marcan SKIP por requerir condiciones especiales (DLL de extension, args de app, red, sintaxis). |
| `examples/audit/test-features/jit-test/bench-jit.ps1` | Benchmark de compilación JIT (`bench5000.clsx`, 5000 llamadas) con timing por fase (`CLS_JIT_TIMING=1`); corrida 1 = cache miss, corrida 2 = cacheada. |
| `examples/audit/benchmarks-modules/run-bench.ps1` | Benchmarks CLS con múltiples módulos (walker vs JIT) sobre `examples/benchmarks-modules`. |
| `examples/audit/benchmark/run-all.cmd` | Ejecuta los tres benchmarks con los mismos parámetros: CLS (JIT, `scripts/clx.cmd`), Node.js (`benchmark.js`) y Python (`benchmark.py`). |
| `examples/audit/fase-1/_runner.ps1` (y `fase-1-r2`, `fase-2`, `fase-2-r2`) | Runner de re-auditoría por fase: captura stdout/stderr a archivos UTF-8, exit code, tiempo y detección de cuelgues (mata el proceso si supera `-HangTimeoutMs`). |
| `examples/audit/release/_release-*.ps1` | Scripts de la auditoría de release (builds, caché, CLI, DX, libs). |
| `examples/audit/validacion/_runner.ps1`, `B-floats.ps1` | Auditoría de validación (runner genérico + caso floats). |
| `examples/audit/migracion/_run.ps1` | Smokes de migración (`smoke-*.clsx`). |
| `examples/jit-examples/run-jit.ps1` | Ejecuta los ejemplos JIT-only **exclusivamente** con `clx run --jit`; falla si el walker no pasa por usar walker. |

## Ejemplos de uso

- `examples/hello` — proyecto mínimo (`cls.json` + `src/main.clsx`).
- `examples/dev-quest` — proyecto de ejemplo con módulos (`main.clsx`,
  `modelo.clsx`, `estadisticas.clsx`, `frases.clsx`).
- `examples/jit-examples` — ejemplos **JIT-only** (validados solo con
  `clx run --jit`): `modules/` demuestra `import ... as`, `from ... import`
  e `include` con múltiples módulos.

## Benchmark

`examples/audit/benchmark/` compara CLS vs Node.js vs Python:
`benchmark.clsx`, `benchmark.js`, `benchmark.py` + `run-all.cmd`
(cada script usa los mismos parámetros `N_*` y la misma lógica).