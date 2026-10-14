# Reporte de regresiones — prueba #3: Brainfuck

> Formato de cierre por prueba. Registra **qué regresiones se encontraron** y
> **qué soluciones se aplicaron**. Documentar el fallo es obligatorio
> (`examples/patrones.md` §5.2.10).

- **Prueba:** #3 Brainfuck (`examples/brainfuck/`) — intérprete CLS puro vs
  oráculo C (FFI), comparación byte a byte + bench.
- **Roles:** dev-pruebas → dev-correcciones → abogado (gate).
- **Veredicto del abogado:** **APROBADO** (verificación independiente).

---

## Regresiones encontradas y solución

| ID | Feature | Síntoma | Causa raíz | Solución | Estado |
|----|---------|---------|------------|----------|--------|
| **B1** | `extension` FFI de nivel superior | **Panic del emisor** (`no entry found for key`) al compilar cualquier `extension` con `function` a nivel raíz. Rompía los `ffi-*.clsx` del repo. `clx check` pasaba. | `collect_function` omite las funciones nativas (`f.native.is_some()`) → no entran en `func_types`; el bucle de `emit` indexaba `self.func_types[&f.name]` para **toda** `FunctionDecl`. | `cls-core/src/backend/wasm/engine/emit.rs` (~623): saltar las nativas en el bucle de emisión de funciones CLS (ya se emiten como imports FFI). | ✅ |
| **B2** | Operadores bit a bit `&` `|` con `Int` | **WASM inválido** (`expected i32, found i64`). No existía AND/OR bit a bit; `^ << >> ~ %` y `&& ||` sí. | El lexer colapsaba `&`→`And` y `|`→`Or` (lógicos i32); el emisor siempre trataba `And`/`Or` como cortocircuito lógico. | Nuevas variantes `BitAnd`/`BitOr` (`token.rs`); precedencia `parse_bitwise_or`/`parse_bitwise_and` (`parser.rs`); tipado `Int`/`Bool` (`typeck/binary.rs`); emisión `i64.and`/`i64.or` vs cortocircuito (`emitter/binary.rs`). | ✅ |
| **B3** | `try/catch/finally` | **`finally` no se ejecutaba** en salida normal ni tras `catch` local (solo al propagar fuera del `try`). `15-try-finally.clsx` fallaba. | `emit_try` solo emitía el `finally` en la rama sin `catch`. | `emitter/statements.rs::emit_try` reescrito con bloque de convergencia (`$finally_label`): el `finally` corre **una sola vez** en los tres caminos; con `catch`, re-lanza tras el `finally`. | ✅ |
| **B4** | Carga de librería FFI | Fallo de `LoadLibrary` **no era fatal**: imprimía el error, devolvía `0` y **exit 0** (falso éxito). | Los wrappers FFI capturaban el `Err` y solo hacían `eprintln!`. | `wasmtime_rt.rs` y `wasmi_rt.rs` devuelven `Err` (trap / `wasmi::Error`) con mensaje claro → trace completo + exit ≠ 0. | ✅ |

## Regresiones de infraestructura (encontradas de paso)

| ID | Área | Síntoma | Causa raíz | Solución | Estado |
|----|------|---------|------------|----------|--------|
| **B5** | Caché JIT (`cls-core/build.rs`) | El caché servía módulos **stale** al editar el backend: `hash_dir` no recursaba en subcarpetas (`emitter/`, `engine/`), así que `BACKEND_HASH` no cambiaba. Enmascaró temporalmente a B3. | `hash_dir` no recorría subdirectorios. | `build.rs::hash_dir` ahora **recursa**. Verificado: editar `emitter/strings.rs` invalida el caché. | ✅ |
| **B6** | Conversiones (`int(string)`) | `int("abc")` / `int("12x")` devolvían `0` en silencio. | Las internals ya exponen `__intr_parse_error_get` y `host_parse_int` devuelve `Err`, pero el emisor ignoraba el flag. | `emit_to_int` comprueba el flag y lanza excepción clara (wasmtime). `int("12")`→12; `int("abc")`→error. En wasmi degrada a 0 (preexistente). | ✅ |

## Gaps de lenguaje (no regresiones; rodeados y documentados)

- **Sin `ord`/`chr`** ni manera de obtener el byte de un `String` de 1 char →
  rodeado con FFI (`bf_ord`).
- **Sin allocador/buffer en CLS**; `CPtr`/byte→`String` no disponible → la I/O de
  bytes usa helper C. `fopen`/`fgetc`/`fclose` con `CPtr` sí funcionan.
- **`CString` de retorno se trunca en NUL** (semántica C `strlen`): para bytes
  crudos usar `CArray` (`bf_read_bytes`). Documentado en `docs/lenguaje/extension.md`.
- **Ruta del `.dll` FFI relativa al CWD** (no al fuente/módulo): dificulta módulos
  FFI reutilizables en `/modules`. Documentado.
- **`Array.pop()` devuelve el array** (semántica no-JS, documentada) → usar
  `a[a.length-1]` + `pop()`.
- **`finally` con `return`/`break`/`continue`** que sale del `try`/`catch` no
  ejecuta los `finally` pendientes (codegen mayor, fuera de alcance).

## Cambios de lenguaje (semántica)

- **`&` y `|` de un solo carácter** pasan de lógicos (`And`/`Or`) a **bit a bit**
  (`BitAnd`/`BitOr`). `&&`/`||` **intactos**. No hay usos existentes que rompan.

## Verificación (al cierre)

- `cargo test --workspace`: **143 passed / 0 failed**.
- `examples/test/jit-test/run-tests.ps1`: **55 PASS / 0 FAIL**.
- `examples/test/jit-test/run-availible.ps1`: **24 PASS / 0 FAIL / 1 SKIP**.
- Brainfuck (sin el workaround `when`): `RESULTADO: OK`, exit 0.
- Abogado: verificó los 4 repros, forzó divergencias (el runner las detecta),
  confirmó **cero `host_calls` nuevos**, **sin exponer punteros**, prueba
  autocontenida/reproducible → **APROBADO**.

## Archivos tocados (por las correcciones)

`cls-core/src/backend/wasm/engine/emit.rs`,
`cls-core/src/backend/wasm/emitter/{statements,binary,convert}.rs`,
`cls-core/src/frontend/{lexer/token,parser}.rs`,
`cls-core/src/middleware/typeck/binary.rs`,
`cls-jit/src/{wasmtime_rt,wasmi_rt}.rs`,
`cls-core/build.rs`,
`examples/test/jit-test/availible/15-try-finally.expect` (oráculo regenerado:
ahora refleja el comportamiento correcto del `finally`),
`examples/test/jit-examples/ffi-*.clsx` (rutas `.dll` corregidas).
