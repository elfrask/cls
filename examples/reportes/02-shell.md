# Reporte de regresiones — prueba #2: Shell POSIX

> Cierre por prueba: **qué regresiones se encontraron** y **qué soluciones se
> aplicaron** (`examples/patrones.md` §5.2.10).

- **Prueba:** #2 Shell POSIX (`examples/clsh/`) — intérprete de un subconjunto
  POSIX en CLS, comparado **byte a byte** (stdout+stderr+exit) contra **`dash`**.
- **Roles:** dev-pruebas → dev-correcciones → Juez (gate).
- **Veredicto del Juez:** **APROBADO** (reprodujo todo; destapó además un bug
  preexistente R5 que se corrigió después).

---

## Resultado

- **22/25 tests byte-idénticos a `dash`** (builtins, expansión `$x`/`${x}`/`$?`,
  comillas/escapes, `$(cmd)` anidada, `; && ||`, redirecciones `> >> < 2>`,
  `if/elif/else`, `while`, `for in`, UTF-8/emoji, exit codes).
- **3 divergencias de arquitectura (no del lenguaje), a propósito rojas:**
  - **D1** `read a b` (reparto de campos vs "el resto al último").
  - **D2** subshell `( … )` no aísla (CLS no tiene `fork`/procesos).
  - **D3** pipe + `read` (el pipe no aísla el estado).

## Regresiones encontradas y solución

| ID | Feature | Síntoma | Causa raíz | Solución | Estado |
|----|---------|---------|------------|----------|--------|
| **R1** | Lexer/parser de strings | Un literal que **termina en `$`** (`"$"`, `"x$"`, `"$$"`, `"\$"`) **cuelga el compilador** (loop infinito) en `check` y `run` | `parse_string_interpolation`: la rama de texto normal no avanzaba si el `$` era el último char (el `while` no ejecutaba cuerpo) | Consumir el `$` literal cuando el texto queda vacío, garantizando progreso (`parser.rs`) | ✅ |
| **R2** | Comparación de `String` | `>`, `<`, `>=`, `<=` **invertidos** (comparaban el i64 empaquetado `ptr<<32\|len`) | El emisor comparaba la representación, no el contenido | Comparación **lexicográfica por bytes inline en WASM** (`i32.load8_u`, prefijo por longitud; semántica `memcmp`), sin host_calls ni punteros (`emitter/binary.rs`) | ✅ |
| **R3** | Scoping de `var` | El cuerpo de una función era **un solo scope**: dos `var x` en bloques hermanos → error de duplicado | `check_block` no hacía push/pop de scope; el emisor mapeaba un nombre a un único local | **Scope por bloque** en typeck (`statements.rs`) **y** emisor (locales frescos + `enter/exit_var_scope` en `mod.rs`/`statements.rs`/`foreach.rs`), respetando shadowing. `when` sin scope a propósito | ✅ |
| **R4** | FFI retorno `CString` | `() -> CString` no asignable a `String`; la presencia de un param `CString` corrompía la inferencia de **todo** el retorno | `map_native_ret` no mapeaba `CString`; el `Named` se sustituía como genérico desde los args | Mapear los escalares C de forma **independiente** de los params (`CString→String`, `CInt/CLong/…→Int`, `CDouble→Float`, `CBool→Bool`) (`typeck/types.rs`) | ✅ |
| **R5** | Igualdad de `String` | `"" == "hola"` → **true** (falso positivo) con cadena vacía | `__intr_str_eq` devolvía `1` si `a_ptr == b_ptr` **antes** de comparar longitudes; el literal `""` comparte puntero con el siguiente | Comparar **longitud primero**; el atajo de puntero queda después (`cls-internals/wasm/src/strings.rs`, `cls-jit/src/host.rs`) | ✅ |

## Extra (rompía el build, no era de los 4 bugs)

- El commit del sandbox (`0f28b1f`) agregó un campo a `HostState` y **dejó `nodos/clxb/src/engine.rs` sin inicializarlo** → `cargo test --workspace` fallaba en `clxb`. Se añadió `limits: None`.

## Gaps / bloqueos

- **Comandos externos**: `msvcrt!system()` funciona pero **no es capturable** sin un buffer de bytes crudos (allocator/`CArray` con puntero, que CLS no expone). Por eso el corpus se limita a **builtins + expansión**.
- **Sin `fork`/procesos** → subshell y pipes reales (D2/D3) no son posibles hoy.
- **`clx check` no marca variables indefinidas** (preexistente; lo detecta `run`).
- **`pwd`** excluido del diff (formato MSYS `/c/...` vs win32 `C:\...`).
- `Array.pop()` devuelve el array; sin `ord`/`chr`.

## Verificación (al cierre)

- `examples/clsh/run.ps1`: **22/25** (D1–D3 rojas a propósito; inyectar un test extra hace bajar a 4/26 → el runner falla de verdad).
- `cargo test --workspace`: **0 fallos** (incluye `clxb`).
- `jit-test/run-tests.ps1`: **55 PASS / 0 FAIL** · `run-availible.ps1`: **24 PASS / 1 SKIP**.
- `conformance/run.ps1`: **2302/2302** + fuzz **20000/20000**.
- `brainfuck/run.ps1`: **OK**.
- Juez: verificó R1 (sin cuelgue), R2 (13/13 vs `strcmp` byte a byte), R3 (shadowing/scope + WASM válido), R4 (combinaciones), reglas (cero `host_calls`, sin punteros), y que D1–D3 son arquitectura → **APROBADO**.

## Archivos tocados (por las correcciones)

`cls-core/src/frontend/parser.rs` (R1),
`cls-core/src/backend/wasm/emitter/binary.rs` (R2),
`cls-core/src/middleware/typeck/statements.rs`, `cls-core/src/backend/wasm/emitter/{mod,statements,foreach}.rs` (R3),
`cls-core/src/middleware/typeck/types.rs` (R4),
`cls-internals/wasm/src/strings.rs`, `cls-jit/src/host.rs` (R5),
`nodos/clxb/src/engine.rs` (`limits: None`).
