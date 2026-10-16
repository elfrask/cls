# Regresiones de CLS halladas por la prueba Brainfuck

Repros mínimos, autocontenidos, de bugs del lenguaje/compilador JIT detectados
al construir `examples/brainfuck/`. **Todos los comandos se ejecutan desde la
raíz del repo.**

Común: `target\debug\clx.exe run examples\brainfuck\findings\<archivo>.clsx`

---

## 1. `extension` FFI a nivel superior → panic del emisor

- **Archivo:** `01-ffi-toplevel-extension.clsx` (contraste que sí funciona:
  `01b-ffi-when-works.clsx`)
- **Feature:** FFI `extension` (declaración a nivel de módulo).
- **Repro:**
  ```
  target\debug\clx.exe run examples\brainfuck\findings\01-ffi-toplevel-extension.clsx
  ```
- **Esperado:** `4`
- **Obtenido:** exit **101**, panic de Rust:
  ```
  thread 'main' panicked at cls-core\src\backend\wasm\engine\emit.rs:623:52:
  no entry found for key
  ```
- **Causa (análisis):** `collect_function` ignora las funciones nativas
  (`f.native.is_some()`), así que no las registra en `func_types`; pero el bucle
  de `emit` (emit.rs:621-623) indexa `self.func_types[&f.name]` para **toda**
  `Statement::FunctionDecl` sin saltar las nativas → `panic`.
- **Alcance:** afecta a cualquier `extension` superior con al menos una
  `function`. Un `extension` superior con **solo** `structure` funciona.
  Envolver en `when (os: windows) { ... }` lo evita (por eso los ejemplos
  `ffi-struct-real.clsx`, `ffi-bump4.clsx`, `ffi-own-ret.clsx` del repo están
  rotos hoy). El patrón documentado en `docs/lenguaje/extension.md` es el roto.
- **Fix sugerido:** saltar `f.native.is_some()` en el bucle de emit.rs (o
  registrarlas como imports).

## 2. `&` y `|` (bitwise int) → WASM inválido

- **Archivo:** `02-bitwise-and-or.clsx`
- **Feature:** operadores bit a bit con operandos `Int` (i64).
- **Repro:**
  ```
  target\debug\clx.exe run examples\brainfuck\findings\02-bitwise-and-or.clsx
  ```
- **Esperado:** `2` y `7`.
- **Obtenido:** exit **1**:
  ```
  [JIT] Módulo WASM inválido ...: Invalid input WebAssembly code at offset 2553:
  type mismatch: expected i32, found i64
  ```
- **Causa (análisis):** `&`/`|` se emiten como instrucciones i32 (lógicas) pero
  los operandos son i64. `^`, `<<`, `>>`, `~`, `%` funcionan; `&&`, `||`, `!`
  (lógicos) funcionan.
- **Impacto:** no se puede enmascarar con `& 255` (hay que usar `% 256`).

## 3. `finally` no se ejecuta (salida normal / tras `catch` local)

- **Archivo:** `03-finally-skipped.clsx`
- **Feature:** manejo de excepciones `try/catch/finally`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\brainfuck\findings\03-finally-skipped.clsx
  ```
- **Esperado:** `try`, `finally`, `catch`, `finally`, `n=3`
- **Obtenido:** `try`, `catch`, `n=1` (los tres `finally` se saltan)
- **Causa:** `finally` solo se ejecuta cuando la excepción *propaga fuera* del
  `try` (sin `catch` local que la capture, ver `15-try-finally.clsx` del repo:
  también falla al faltar "finally normal"). En salida normal y tras un `catch`
  local no corre.
- **Impacto:** cualquier cleanup/liberación en `finally` es no-op.

## 4. Carga de librería FFI fallida → error no fatal

- **Archivo:** `04-ffi-lib-missing-continues.clsx`
- **Feature:** error de carga de librería nativa (`extension`).
- **Repro:**
  ```
  target\debug\clx.exe run examples\brainfuck\findings\04-ffi-lib-missing-continues.clsx
  echo $LASTEXITCODE
  ```
- **Esperado:** error de runtime fatal (exit != 0).
- **Obtenido:** se imprime el error en stderr pero la llamada devuelve `0`, el
  programa imprime `result 0` y sale con **exit 0** (falso éxito).
- **Impacto:** un FFI mal configurado (ruta/abi) pasa desapercibido en CI.

## 6. `switch` sobre `String` compara el handle, no el contenido

- **Archivo:** `06-switch-string-compara-handle.clsx`
- **Feature:** `switch`/`case` con valor `String`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\brainfuck\findings\06-switch-string-compara-handle.clsx
  ```
- **Esperado:** las tres clasificaciones iguales (`MOVE`, `MOVE`, `MOVE`).
- **Obtenido:** `MOVE`, `?`, `?` — solo matchea el literal directo (mismo handle
  internado que el `case`); un String construido en runtime (concatenacion,
  substring `s[i]`) con el mismo contenido cae al `default`.
- **Causa:** `emit_switch` (`cls-core/src/backend/wasm/emitter/statements.rs`)
  comparaba con `push_eq(WasTy::I64)` (i64.eq sobre el handle) en vez de
  `__intr_str_eq` (comparacion por contenido, la que usa `==`).
- **Alcance:** hallado refactorizando el `cmd_of` de Brainfuck a `switch` (por eso
  `cmd_of` despachaba por el byte con `ord`, no por el String). No bloquea el test.
- **Estado:** ✅ **RESUELTO** — `emit_switch` elige la comparacion por el tipo del
  valor: String por contenido (`__intr_str_eq`), Int/enum por `i64.eq`, Bool/Char/
  Float por su propio operando (`i32.eq`/`f64.eq`). `cmd_of` volvio a despachar por
  el caracter natural (`switch (ch) { case (">") ... }`).

---

## Gaps (rodeables, sin crash)

- **5. `CString` de retorno se trunca en NUL** — `05-gap-cstring-nul.clsx`:
  `nul.bin` = `41 00 42`; el `String` resultante mide 1 (debería ser 3). No hay
  vía byte-safe sin helper (se usó `bf_read_bytes -> CArray`).
- **Sin `ord`/`chr`** — no hay forma de obtener el byte de un `String` de 1 char
  (lo usa `,` de BF). Se rodeó con FFI (`bf_ord`).
- **`int(string)` inválido**: ~~0 silencioso~~ **CORREGIDO** — ahora falla con
  error claro (`int("abc")`, `int("12x")` → exit ≠ 0) en wasmtime.
- **`Array.pop()` devuelve el array**, no el elemento.
- **Path del `.dll` FFI relativo al CWD**, no al archivo/módulo.
