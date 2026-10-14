# Reporte de regresiones — prueba #1: Suite de conformidad + differential vs C

> Cierre por prueba: **qué regresiones se encontraron** y **qué soluciones se
> aplicaron** (`examples/patrones.md` §5.2.10).

- **Prueba:** #1 Conformidad (`examples/conformance/`) — CLS puro vs oráculo C
  (FFI) + fuzzer determinista.
- **Roles:** dev-pruebas → dev-correcciones → abogado (gate).
- **Veredicto del abogado:** **APROBADO** (verificación independiente, incluido
  el contraste `finally` con Node/JS y un test anti-falso-verde).

---

## Resultado de la suite

- **2302/2302** casos coinciden con C (arith/bitwise, floats, strings+UTF-8,
  control/recursión, closures, arrays/records/tuplas, excepciones).
- **Fuzzer determinista: 20000/20000** (semilla fija `88172645463325252`).
- **Caché fría == caliente** (sustituto del oráculo 3, que no existe en CLS).

## Regresiones encontradas y solución

| ID | Eje | Síntoma | Causa raíz | Solución | Estado |
|----|-----|---------|------------|----------|--------|
| **C1** | Excepciones / flujo | **`finally` no corría** ante salida abrupta del `try` (`return`, `break`, `continue`): `fin_return=0/0/0` en vez de `1/1/3`. (Sí corría en salida normal, por excepción que propaga, y tras `catch`.) | Las salidas abruptas emitían `Return`/`Br` directos, **saltándose** el bloque `$finally_label`. | Emisor: pila `finally_stack` (`mod.rs`) + push/pop en `emit_try` (`statements.rs`); `Return`/`Break`/`Continue` que cruzan un `finally` guardan su intención (`exit_kind`) + valor y saltan al `finally` más interno; tras el `finally`, `emit_deferred_exit_dispatch` completa o salta al siguiente `finally`. El `catch` se envuelve para diferir excepciones lanzadas desde él. | ✅ |
| **C2** | FFI (`extension`) | Args **`CDouble` llegaban mal a C** (basura denormal): `o_binop_f(+,2.5,0.5)` ≠ 3. Ninguna función nativa con params `double` funcionaba (`libm`, aritmética FP...). | El dispatcher (`native_backend.rs`) invocaba con **una sola firma** `extern "C" fn(i64,…,i64)`: todos los args por registros enteros; en x86_64 un `double` de C va en **XMM**. | Dispatcher reescrito con **libffi**: firma real por slot (`i64`→registro entero, `f64`→registro FP). Se eliminó la tabla `call_typed_0..16` (~400 LOC). Dependencia `libffi = "3"` (`cls-runtime/Cargo.toml`). | ✅ |
| **C3** | FFI (`extension`) | El **retorno `CString` no era byte-transparente**: bytes UTF-8 inválidos se normalizaban a U+FFFD (`"汉字"[0]` medía 3, no 1). | `conv_ret` usaba `CStr::to_string_lossy()`. | Canal interno de bytes: `NativeRet::{Value,Bytes}` (`ffi.rs`); `call_function_raw` devuelve los bytes crudos del `char*`; los wrappers JIT los escriben tal cual (`host::write_packed_bytes` en `wasmtime_rt.rs`/`wasmi_rt.rs`). Sin `unsafe` de UTF-8 ni exponer punteros. | ✅ |

## Gaps de lenguaje (no regresiones; rodeados y documentados)

- **Sin `ord`/`chr`/byte-de-char**: `s[i]` devuelve `String`, no entero.
- **`Array.pop()`/`shift()` devuelven el array mutado**, no el elemento.
- **Ruta del `.dll` FFI relativa al CWD** (no al fuente/módulo).
- **wasmi**: sin excepciones y con bugs preexistentes de FFI/host → solo informativo.

## Bug preexistente detectado (fuera de alcance)

- `function f() -> float { return 1; }` emite **WASM inválido**
  (`expected f64, found i64`): la promoción `int→float` no se aplica al `return`
  de un literal. No lo introdujo esta tanda; queda anotado.

## Semántica verificada vs C (documentada; NO son bugs)

División truncada a cero (`i64.div_s`), `%` con signo del dividendo, overflow
**wrap** de 64 bits, shifts enmascarados `mod 64` (`>>` aritmético), `& | ^ ~`
bit a bit, promoción `int→float` implícita/exacta (<2^53), comparación IEEE-754.

## Verificación (al cierre)

- `examples/conformance/run.ps1`: **2302/2302** + fuzz **20000/20000**, fría==caliente, 3 findings resueltos, `RESULTADO GLOBAL: OK`.
- `cargo test --workspace`: **143 passed / 0 failed** (compilado desde cero con libffi).
- `examples/test/jit-test/run-tests.ps1`: **55 PASS / 0 FAIL**.
- `run-availible.ps1`: **24 PASS / 0 FAIL / 1 SKIP**.
- `examples/brainfuck`: **OK** (byte a byte).
- Abogado: anti-falso-verde OK (inyectó divergencias → el runner las detecta),
  `finally` 11/11 vs Node, FFI int/mixto 15/15, **cero `host_calls` nuevos**, sin
  punteros, oráculo externo/determinista → **APROBADO**.

## Archivos tocados (por las correcciones)

`cls-core/src/backend/wasm/emitter/{statements,mod}.rs`,
`cls-runtime/src/native_backend.rs`,
`cls-runtime/src/ffi.rs`,
`cls-runtime/Cargo.toml` (libffi),
`cls-jit/src/{wasmtime_rt,wasmi_rt,host}.rs`.

## Pendientes (mejoras, no bloqueantes)

- Retirar los workarounds de la suite ahora que C2/C3 están resueltos (firma
  `CDouble` directa; comparación de contenido byte-exacto en slices multibyte).
- Cachear el `Cif` de libffi por firma (hoy se reconstruye por llamada).
- Ampliar cobertura §1: precedencia/asociatividad (expresiones, no dispatch),
  `for`/`for each`, `split`/concat, spread.
