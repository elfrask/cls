# Regresiones de CLS halladas por la suite de conformidad

Repros mínimos, autocontenidos, de bugs del lenguaje/JIT/FFI detectados al
construir `examples/conformance/`. **Todos los comandos se ejecutan desde la raíz
del repo.** Los findings #2 y #3 necesitan `oracle.dll` (lo compila `run.ps1` o
`gcc -shared -O2 -o examples\conformance\oracle\oracle.dll examples\conformance\oracle\oracle.c`).

> **Estado: los 6 findings fueron RESUELTOS** por el desarrollador de
> correcciones (ver `examples/reportes/01-conformance.md`). Se conservan los
> repros como historia; hoy dan el comportamiento **esperado** (exit 0).

---

## 1. `finally` NO se ejecuta al salir del `try` por control abrupto (`return`/`break`/`continue`)

- **Archivo:** `01-finally-control-flow.clsx` (puro CLS).
- **Feature:** manejo de excepciones `try/finally`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\01-finally-control-flow.clsx
  ```
- **Esperado:** `fin_return=1 fin_break=1 fin_continue=3` (exit 0).
- **Obtenido:** `fin_return=0 fin_break=0 fin_continue=0` (exit **1**).
- **Alcance (caracterizado):**

  | Salida del `try` | `finally` corre |
  |------------------|-----------------|
  | normal (fin de bloque) | ✅ |
  | excepción que propaga | ✅ |
  | tras `catch` local | ✅ |
  | **`return`** | ❌ **se salta** |
  | **`break`** | ❌ **se salta** |
  | **`continue`** | ❌ **se salta** |

  La suite (`src/main.clsx`, eje *excepciones*) confirma los tres primeros;
  este repro añade los tres abruptos.
- **Tipo:** bug del **JIT/emisión de flujo** (el `br` de `return`/`break`/
  `continue` no pasa por el bloque `finally`), no del typeck.
- **Impacto:** cualquier cleanup/liberación/cierre en `finally` no se ejecuta al
  retornar/cortar el bucle; fuga de recursos y estado inconsistente.
- **Estado:** ✅ **RESUELTO** — el emisor difiere las salidas abruptas
  (`finally_stack` + dispatch de salida diferida): todo `return`/`break`/
  `continue` que cruza un `finally` lo ejecuta antes de saltar. Ver
  `examples/reportes/01-conformance.md`.

## 2. FFI: los argumentos `CDouble` llegan mal a C (ABI roto)

- **Archivo:** `02-ffi-f64-args.clsx`.
- **Feature:** `extension` (FFI a C) con parámetros `CDouble`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\02-ffi-f64-args.clsx
  ```
- **Esperado:** `o_binop_f(+,2.5,0.5)=3` y `o_mix(+,7,2.0)=9`.
- **Obtenido:** `o_binop_f` ≈ `0.5` (basura denormal) y `o_mix=7`; exit **1**.
- **Causa (raíz, `cls-runtime/src/native_backend.rs`):** el dispatcher invoca el
  símbolo con **una sola firma** `unsafe extern "C" fn(i64, i64, …, i64) -> …`
  (`call_typed_0..16`), pasando **todos** los argumentos como `i64` en registros
  enteros. En x86_64 un parámetro `double` de C vive en un registro **XMM**, así
  que C lee basura. En la práctica el **último float procesado** queda en XMM0 y
  el resto de parámetros vale 0 (p. ej. `d_add(a,b)` recibe `b` en el slot de
  `a`). Evidencia adicional (repro directo con un DLL de prueba):
  ```
  d_ident(7.5)   = 7.5    (1 param double "funciona" por casualidad de XMM0)
  d_add(0,3)     = 3      d_add(2.5,0.5) = 0.5
  d3(1,2,3)      = 300    d_sum4(1,2,3,4) = 4
  i_d_add(7,2.0) = 7      (el double tras un int se pierde)
  ```
- **Retorno `CDouble`:** sí funciona.
- **Tipo:** bug del **FFI/JIT** (marshalling de argumentos), no del typeck.
- **Impacto:** **ninguna** función nativa con parámetros `CDouble` funciona
  (aritmética flotante, `libm`, geometría, etc.). Es un bloqueo para cualquier
  oráculo C que use doubles.
- **Workaround en la suite:** el oráculo expone `o_fcmp`/`o_mix_cmp` que reciben
  los operandos y el resultado de CLS **como strings** (`strtod` + comparación
  bit a bit), evitando el paso de `CDouble`. Ver `README.md`.
- **Fix sugerido:** emitir la llamada con la firma real por slot
  (tipos `i64`/`f64` en su registro ABI correcto), p. ej. especializando
  `call_typed_N` por máscara de tipos o usando `libffi`.
- **Estado:** ✅ **RESUELTO** — el dispatcher invoca con **libffi** y la firma
  real por slot (`i64`→registro entero, `f64`→registro FP); se eliminó la tabla
  `call_typed_0..16`. `o_binop_f(+,2.5,0.5)=3`.

## 3. FFI: el retorno `CString` no es byte-transparente (UTF-8 inválido → U+FFFD)

- **Archivo:** `03-cstring-return-utf8.clsx`.
- **Feature:** `extension` (FFI), retorno `char*`/`CString`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\03-cstring-return-utf8.clsx
  ```
- **Esperado:** el primer byte de `"汉字"` (`0xE6`) medido como `String` = **1**.
- **Obtenido:** el `CString` de retorno mide **3** (el U+FFFD `EF BF BD` de
  reemplazo); exit **1**. `o_slice_len` (solo longitud) devuelve 1.
- **Detalle:** `.slice` de CLS **sí** es byte-exacto (mide 1). El bug es solo del
  **retorno CString** al construir el `String`: normaliza bytes no-UTF-8 a
  U+FFFD (familia del corte en NUL ya documentado). No hay forma de recibir bytes
  crudos por `CString`.
- **Tipo:** bug/gap del **FFI** (conversión de retorno), no del typeck.
- **Impacto:** no se pueden transferir bytes binarios desde C sin `CArray`
  (helper explícito). Afecta a parsers, protocolos binarios, hashes, etc.
- **Rodeo:** la suite compara slices multibyte por **longitud** (`o_slice_len`);
  el contenido solo se compara para ASCII. Para bytes crudos usar `CArray`
  (como `bf_read_bytes` del oráculo Brainfuck).
- **Estado:** ✅ **RESUELTO** — el retorno `CString` ahora es **byte-transparente**
  (canal interno de bytes: `NativeRet::Bytes`; los wrappers JIT escriben los
  bytes crudos). `CArray` sigue disponible para buffers binarios.

## 4. Closure que captura la variable de `for each` → valor rancio

- **Archivo:** `04-for-each-closure-capture.clsx` (puro CLS).
- **Feature:** closures + `for each`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\04-for-each-closure-capture.clsx
  ```
- **Esperado:** `seen = "0,1,10,"` (exit 0).
- **Obtenido:** `seen = "0,0,0,"` (exit **1**) — la closure lee siempre el valor
  inicial de la variable del `for each`. Con `for` clasico + `var base = ks[i]`
  de cuerpo la captura es correcta.
- **Tipo:** bug del **JIT** (binding de la variable de iteración única, no por
  iteración; la closure captura por referencia al slot compartido).
- **Estado:** ✅ **RESUELTO** — el emisor de `for each`
  (`cls-core/src/backend/wasm/emitter/foreach.rs`) asigna la variable de
  iteración promovida al heap con un slot NUEVO por iteración (binding por
  iteración). `axis_closures` volvió a usar `for each base in (ks)`.

## 5. `const` de literal ENTERO no es usable en aritmética (tipo literal)

- **Archivo:** `05-const-int-literal-arith.clsx` (puro CLS).
- **Feature:** `const` (inferencia de tipo literal) + operadores aritméticos.
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\05-const-int-literal-arith.clsx
  ```
- **Esperado:** `12` (exit 0).
- **Obtenido:** error de compilación (exit **1**):
  `Operador + no soportado entre Int y 5 (en x + N)`.
- **Causa:** `const N = 5` infiere `Type::Literal(5)`; el chequeo de operadores
  binarios solo aceptaba `Int`/`Float` base, no un tipo literal. (`const` de String
  sí funciona porque los literales de String unifican con `String`.)
- **Alcance:** hallado al extraer las constantes del LCG a `const` en
  `modules/prng`; se dejaron como `var LCG_A: int = ...`. No bloquea la suite.
- **Estado:** ✅ **RESUELTO** — `check_binary`
  (`cls-core/src/middleware/typeck/binary.rs`) opera los literal types como su
  tipo base (`Type::literal_base`). `modules/prng` volvió a `const LCG_A/LCG_C`.

## 6. Closure que devuelve `Bool` (o `Float`) emite WASM inválido

- **Archivo:** `06-closure-return-bool.clsx` (puro CLS).
- **Feature:** arrow/closure + retorno tipado (`Bool`, `Float`).
- **Repro:**
  ```
  target\debug\clx.exe run examples\conformance\findings\06-closure-return-bool.clsx
  ```
- **Esperado:** imprime `yes` / `yes2` (exit 0).
- **Obtenido:** error de compilación (exit **1**):
  ```
  [JIT] Módulo WASM inválido ...: type mismatch: expected i64, found i32
  ```
  Igual con `() -> (s == "ab")` (String) y con una closure que devuelve `Float`
  (`expected i64, found f64`).
- **Causa raíz:** la declaración de toda arrow normalizaba la firma a
  `[capturas:i64, params:i64...] -> i64` (`engine/emit.rs`), y el `engine/functions.rs`
  declaraba todos los params como `i64` con un prologue de
  `f64.reinterpret_i64`. Pero el cuerpo emite el valor en su tipo natural
  (`==` → `i32`, `Float` → `f64`) y el llamador vía handle (`Type::Fun`,
  `emitter/calls.rs`) construye el `call_indirect` con los tipos **reales** del
  callee. Además, una función CLS top-level usa la firma natural, así que el
  handle de arrows y el de funciones divergían.
- **Fix:** la arrow usa la firma **natural** como una función top-level
  (`[capturas:i64, types...] -> was_type(ret)`), sin normalizar params ni retorno;
  se eliminó el prologue de reinterpretación y el helper `override_local`.
  `Int`/`String`/`Any` (i64) no cambian; `Bool` (i32) y `Float` (f64) ahora
  coinciden con el `call_indirect` del llamador.
- **Estado:** ✅ **RESUELTO** — `06` imprime `yes`/`yes2` (exit 0); closures que
  devuelven Int/Float/String/Bool y pasadas como `f: () -> Bool` funcionan.

---

## Gaps de lenguaje (rodeables, sin crash) — relevantes a esta suite

- **Sin `ord`/`chr`/byte-de-char**: no se puede obtener el código de byte de un
  `String` de 1 byte (`s[i]` devuelve un `String`, no un entero). Por eso el
  chequeo de records de la suite evita hashes de claves y usa longitudes, y un
  Shell/SHA-256 necesitarían FFI. Ya reportado por la prueba Brainfuck.
- **`Array.pop()`/`shift()` devuelven el array mutado**, no el elemento (semántica
  no-JS). La suite no depende de su retorno.
- **Ruta del `.dll` FFI relativa al CWD**, no al archivo/módulo: obliga a correr
  desde la raíz del repo. Ya reportado por la prueba Brainfuck.

## Divergencias de semántica vs C (documentadas; NO son bugs)

La suite fija y verifica la semántica real de CLS hoy:

| Tema | CLS (hoy) | Igual a C | Nota |
|------|-----------|-----------|------|
| División entera | truncada hacia cero (`i64.div_s`) | Sí | `-7/3 = -2`. Trap en `/0` y `INT64_MIN/-1` (WASM). |
| `%` con signo | signo del dividendo (`i64.rem_s`) | Sí | `-7%3 = -1`. |
| Overflow `+ - *` | **wrap** de 64 bits | Sí (vía unsigned) | `INT64_MAX+1 = INT64_MIN`. |
| Shifts | enmascaran el desplazamiento `mod 64` | Definido (C es UB) | `1<<64 = 1`. `>>` es **aritmético**. |
| `& \| ^ ~` | bit a bit sobre i64 | Sí | OK (regresión previa ya corregida). |
| Promoción `int->float` | implícita y exacta (<2^53) | Sí | única coerción del lenguaje. |
| Comparación de floats | IEEE-754 (`NaN != NaN`) | Sí | `/0.0` da `inf`; la suite evita NaN por igualdad. |
| `int(string)` inválido | falla (exit≠0) | — | ya corregido; en wasmi degrada a 0. |

## Limitaciones / notas

- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): sin excepciones (compila `try/catch`) y con
  bugs preexistentes de FFI/host; se corre solo como **informativo**.
- El oráculo 3 (`O0` vs `O2`, `optimizationLevel`) **no existe** en CLS; se
  sustituye por **caché fría vs caliente** (`clx clean` entre corridas) en
  `run.ps1`, que detecta no-determinismo / bugs de caché.
