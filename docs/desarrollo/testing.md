# Testing

## Unit tests por crate

Todos los crates llevan tests unitarios inline (`#[cfg(test)]`) y tests de
integración en `tests/`. Se ejecutan con:

```ps
cargo test                 # todo el workspace
cargo test -p clx          # solo un crate
```

| Crate | Dónde | Tests |
|---|---|---|
| `cls-core` | `src/frontend/lexer.rs`, `src/frontend/parser.rs`, `src/middleware/typeck/tests.rs`, `src/error/mod.rs` | span/merge/fábricas de error, lexer, parser, typeck |
| `cls-runtime` | `src/value.rs`, `src/vfs/*`, `src/error_report.rs`, `src/ffi.rs` | valores, VFS, formato de errores, FFI |
| `cls-jit` | `tests/` — `wasmi_smoke.rs`, `host_call.rs`, `exports.rs`, `debug_kinds.rs` | **`wasmi_smoke` requiere la feature `wasmi-runtime`** |
| `cls-internals` | `tests/internals_test.rs` | paridad host ↔ internals (WASM fusionado) |
| `nodos/clxb` | `tests/embed.rs` | embedding (`ClsEngine`): exports, arrays/records, `run_main`/`eval`, `OutputSink`, SDK |

```ps
cargo test -p clxb
```

## Suite de features (JIT): `examples/test/jit-test/`

| Ruta | Contenido |
|---|---|
| `units/` | Unit tests + oráculos `.expect` (`a1`–`a11`, `b1`–`b9`, `f64arr`, `deadflow`, `mod-*`, `str-*`, `ffi-*`, `synerr`, ...) y módulos auxiliares (`modfixes/`, `export-list-mod/`, `ffi-export-mod/`) |
| `availible/` | Features disponibles del JIT (`01-operadores.clsx` … `25-bitops.clsx`) |
| `feats/`, `print-test/` | Pruebas adicionales |

Runners (PowerShell):

| Script | Propósito |
|---|---|
| `examples/test/jit-test/run-tests.ps1` | Corre `units/` y compara la salida contra el oráculo `.expect` |
| `examples/test/jit-test/run-availible.ps1` | Corre `availible/` y compara contra `.expect` |
| `examples/test/jit-test/generate-oracles.ps1` | Regenera los `.expect` desde la salida actual del JIT |
| `examples/test/jit-test/bench-jit.ps1` | Benchmark de compilación JIT (cache miss vs cacheada) |

Los runners usan **`target/debug/clx.exe`**: compilá antes con
`cargo build --bin clx`. Estado actual: `units` **85 PASS**, `availible`
**24 PASS + 1 SKIP** (0 FAIL).

Convención: los oráculos `.expect` son la salida **conocida-correcta del JIT**
(el walker se eliminó). Un test de programa debe declarar
`function main(args: String[]) -> int` **anotado** (el JIT lo exige). Los tests
de error intencional se validan por `exit != 0` + stderr.

## Suites diferenciales (`examples/`)

Comparan CLS contra un **oráculo externo** (no contra otro intérprete de CLS):

| Suite | Oráculo | Runner |
|---|---|---|
| `conformance` | C vía FFI + specs | `run.ps1` |
| `clsh` | `dash` byte a byte | `run.ps1` |
| `brainfuck` | intérprete BF en C | `run.ps1` |
| `sqlite-orm` | Python `sqlite3` | `run.ps1` |
| `dns` | oráculo Python independiente | `run.ps1` |
| `cmx-templates` | golden + Jinja2 | `run.ps1` |
| `pkgcheck` | `hashlib` + `certutil` | `verify.ps1` |
| `snake` | `.final` + oráculo Python | `replay.ps1` |
| `scheme` | `.out` (+ `guile`) | `diff.ps1` |
| `lang-features` | golden + negativos | `run.ps1` |

Los reportes de cierre viven en `examples/reportes/`.

## `clx test`

`cls.json` acepta `testEntry` (array de rutas o globs `*`/`**`/`?`). El
subcomando:

```ps
clx test                       # usa testEntry
clx test tests/foo.clsx        # archivos/globs explícitos
clx test --filter user         # filtra por subcadena de ruta
clx test --list                # solo lista los tests descubiertos
```

Hoy cada archivo se ejecuta con el JIT y su **exit code** decide pass/fail. Los
intrínsecos de test en el lenguaje (`test()`, aserciones, nombres reservados)
son la **fase final** — propuestas en `docs/lenguaje/testing.md`.

`clx test` aplica `pathModules` del `cls.json` del CWD y del proyecto de cada
test, igual que `clx run`; sin eso, los tests no resuelven `/modules`.

## Tombstone (`cls-repositories/tombstone`)

Requiere los shims de `io`, `socket`, `sqlite` (y `csprng`/`argon2` para auth).
Desde la raíz del repo CLS:

```powershell
powershell -File cls-repositories\tombstone\tests\unit\run.ps1
powershell -File cls-repositories\tombstone\tests\db\run.ps1
powershell -File cls-repositories\tombstone\tests\integration\http_smoke.ps1
```

Estado actual: **13 tests unitarios**, **75 checks DB** y **71 checks HTTP**
(0 FAIL). El runner de integración levanta el servidor real y usa `curl`.

## Ejemplos

- `examples/test/hello` — proyecto mínimo (`cls.json` + `src/main.clsx`).
- `examples/test/jit-examples` — ejemplos **JIT-only** (validados solo con
  `clx run`): FFI (`ffi-*`), módulos (`modules/`) y aislamiento por `cls.json`
  (`modules-isolation/`). Runner: `run-jit.ps1`.
- `examples/test/benchmark-langs` — CLS vs C++ / JS / Python / Rust; ver
  `REPORTE-BENCHMARKS.md`.

> Nota: `examples/` se usa para **proyectos que ponen a prueba CLS**; los
> ejemplos viejos de auditoría (`examples/audit/`) se eliminaron.
