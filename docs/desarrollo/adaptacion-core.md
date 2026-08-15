# Fase 3 — Adaptación del core WASM al enfoque de internals precompilados

> **Estado**: PLAN. **Depende de Fases 1 y 2** (backend modular + crate `cls-internals`).
> **Objetivo**: el emisor (`cls-core/src/backend/wasm/`) deja de emitir
> `call env.arr_push`/`env.str_concat`/`env.math_sqrt` y pasa a llamar funciones
> WASM internas (linkeadas del módulo de internals), eliminando el traspaso
> WASM↔host en los hot paths.

## 3.1 Cambios en el emisor (por módulo del refactor)

| Módulo (Fase 1) | Cambio |
|---|---|
| `emitter/modules.rs` | `emit_math_call`: `sqrt/floor/ceil/round/abs/min/max` → **instrucciones nativas** (`F64Sqrt`, `F64Floor`, `F64Ceil`, `F64Nearest`, `F64Abs`, `F64Min/Max`) — cero llamadas. `sin/cos/tan/log/pow` → `call __intr_math_sin` etc. (según decisión de precisión de Fase 2) |
| `emitter/containers.rs` | `push/pop/shift/unshift/index_of/includes/reverse/join` → `call __intr_arr_*`. **Además**: el caso común de `push` (capacidad suficiente) se emite **inline** (store del slot + len++) y solo el realloc llama `__intr_arr_realloc` (que internamente hace `memory.grow`). El `writeback_array` existente se conserva |
| `emitter/strings.rs` | `StrConcat`/`StrLength`/`StrUpper`/etc. → `call __intr_str_*`. `len` se emite inline (`(bits & 0xffff_ffff)`) |
| `emitter/binary.rs` | `StrConcat` en `+` de strings → `__intr_str_concat`; `Fmod` → `__intr_math_fmod` |
| `emitter/calls.rs` | `len/toString/int/float/bool` sobre primitivos → `__intr_*` (o inline). **Shadow call stack**: ver 3.3 |
| `engine/mod.rs` | Inyección del módulo de internals: fusión de secciones en el módulo CLS (opción (a) de Fase 2) o registro del import `"internals"` (opción (b)) |
| `engine/globals.rs` | `build_allocator`/`build_load_str` se comparten con el módulo interno (misma memoria) |

## 3.2 Contrato de imports restantes (`env.*` que sobreviven)

Tras la adaptación, el módulo CLS solo importa del host:

```
print_int/float/bool/char/str, print_raw
input
fs_read/write/exists/rm/mkdir/list_dir/cwd
http_get/post
process_*, os_*, time_*, random_*
trap, throw (payload), exit, sleep
host_call (canal de intrinsics del nodo — bindings)
```

Todo lo demás (`arr_*`, `str_*`, `record_*`, `math_*`, conversiones) desaparece
de los imports. Los hosts (`cls-jit/src/host.rs`, `wasmtime_rt.rs`, `wasmi_rt.rs`)
se adelgazan al subconjunto de I/O + errores.

## 3.3 Shadow call stack en WASM (fn_enter/fn_exit)

Hoy cada llamada CLS emite 3 host calls (`fn_enter`, `fn_call_site`, `fn_exit`)
para mantener el trace de errores — es el costo dominante de fib (x2091).

**Diseño**:
- Región dedicada en la memoria lineal: `[shadow_ptr:i32][frames...]` con entradas
  de 16 bytes `(fn_idx:i32, line:i32, col:i32, call_line:i32)` (o 24B con nombre).
- `fn_enter` → 4-5 instrucciones WASM (store de la entrada + `shadow_ptr += 16`).
- `fn_exit` → 1 instrucción (`shadow_ptr -= 16`).
- `fn_call_site` → 1 store del span pendiente en un slot fijo.
- El host lee la región **solo al momento del trap/error** (una lectura), la
  de-shiftea con `HostState.modules` y construye el `ErrorReport.stack` exacto.

**Compatibilidad**: `error_report.rs` y el formateo NO cambian; los tests de
errores (`err-divzero-nested`, `err-error-en-modulo`, `stress-recursion`) son la
red de seguridad de que el trace queda idéntico.

**Flag**: `WasmBackendOptions.trace_calls` (default true) — `clx run --release`
puede desactivarlo aceptando perder el trace (documentar en `docs/runtime/errores.md`).

## 3.4 Unificación de `expr_display` (deuda A-4)

Con el backend ya modular, se unifica el formateo de expresiones en UNA sola
fuente (`frontend/ast/display.rs` — movido en Fase 1): `typeck.expr_short_display`
y `wasm.helpers.expr_display` pasan a delegar ahí. Elimina ~280 líneas duplicadas
y garantiza mensajes idénticos en typeck y emisor.

## 3.5 Pasos de implementación

| Paso | Contenido | Verificación |
|---|---|---|
| 1 | Instrucciones nativas de math (sqrt/floor/ceil/round/abs/min/max) | `math` benchmark + suite 11-stdlib |
| 2 | Strings: len inline + `__intr_str_*` | `04-string` benchmark + suite 02/03-strings |
| 3 | Arrays: `__intr_arr_*` + push inline con realloc interno | `03-array` + suite 04-arrays |
| 4 | Records/tuplas: `__intr_record_*` | suite 06-records, 05-tuplas |
| 5 | Conversiones/intrinsics puros | suite 14-intrinsics, 13-stdlib |
| 6 | Shadow call stack WASM | tests de errores + suites completas |
| 7 | Adelgazar hosts (eliminar los `env.*` no usados) | `cargo check` + grep de imports en WAT |
| 8 | Flag `trace_calls` | `clx run --release` + doc |

## 3.6 Criterios de aceptación

- [ ] `CLS_DUMP_WAT=1` sobre `benchmark-langs/cls/*.clsx` NO muestra imports
  `env.arr_*`/`env.str_*`/`env.record_*`/`env.math_*`.
- [ ] Suites: 25 PASS + 20 PASS + tests del workspace (sin regresiones;
  `host_call_wasmi` pre-existente se revisa aquí — los internals deben funcionar
  en wasmi).
- [ ] Benchmarks: fib ≤ 300ms, arrays ≤ 40ms, strings ≤ 10ms, math ≤ 25ms
  (umbrales de `crate-internos.md`).
- [ ] Errores de runtime con trace idéntico al actual (shadow stack).
- [ ] Bindings C/Python/JS re-verificados (no tocan el backend, pero el motor
  compartido sí — correr sus suites).
