# Fase 2 — Crate `cls-internals`: módulos internos e intrinsics precompilados

> **Estado**: PLAN. **Depende de Fase 1** (backend modularizado).
> **Motivación**: los benchmarks (`REPORTE-BENCHMARKS.md`) muestran que cada
> operación de array/string/math/record cruza la frontera WASM↔host
> (`env.arr_push`, `env.str_concat`, `env.math_sqrt`, ...) costando ~100-500ns
> por cruce: fib(30) x2091 vs C++, arrays x2156, strings x1926, math x44.
> La aritmética pura (0 cruces) ya es competitiva (90.7ms vs 91 C++).

## Objetivo

Crear un crate `cls-internals` que contenga los **módulos internos e intrinsics
implementados como funciones WASM precompiladas**, de modo que el emisor las
**linkee dentro del módulo CLS** (misma memoria lineal, llamadas WASM puras) y
se **elimine el traspaso hostcall** para todo lo que no necesita el host
(I/O, reloj, entropía, trap).

## Qué se mueve a WASM y qué queda en el host

### ✅ A WASM (determinista, no necesita el host)

| Área actual | Host functions de hoy | Nota |
|---|---|---|
| Arrays | `arr_push/pop/shift/unshift/index_of/includes/reverse/len/get/set/join/...` | El dato ya vive en la memoria lineal |
| Strings | `str_len/str_concat/str_repr/str_upper/lower/trim/starts_with/ends_with/contains/...` | Manipulación pura de bytes |
| Records | `record_get/set/len/keys/values/...` | Hash en WASM (Fnv/SipHash) |
| Tuplas | operaciones de tuplas | Layout ya fijado |
| Math determinista | `math_sqrt/floor/ceil/round/abs/min/max` | **Instrucciones nativas**: `f64.sqrt/floor/ceil/nearest/abs` |
| Conversiones | `parse_int/float/bool`, `str_*` | Parseo en WASM |
| Intrinsics puros | `len`, `toString`, `int`, `float`, `bool`, `type` (sin objetos) | Lógica local |
| `pow` | `math_pow` | WASM no tiene `pow` → `exp2(log2)` o Newton |
| `sin/cos/tan/log` | `math_sin/cos/tan/log` | Polinomios minimax (decisión de precisión — ver §Riesgos) |

### 🔗 Quedan como host calls (inherentes al host)

| Área | Por qué sigue en host |
|---|---|
| `print/input` | stdout/stdin del proceso |
| `fs.*`, `http.*` | I/O del sistema operativo |
| `process.exit/cwd/env/args/pid` | Estado del proceso |
| `time.*`, `os.uptime` | Reloj/sistema |
| `random.*` | Entropía del SO |
| `trap/throw` | Mecanismo de errores |
| `exit/sleep` | Control del proceso |
| `json.parse/stringify` | Opcional: se puede mover a WASM (parseo determinista) |
| realloc (`memory.grow` + copia) | Se puede emitir **inline** en el emisor (Fase 3) |

## Diseño del crate

```
cls-internals/
├── Cargo.toml
├── build.rs               # compila los internals a WASM (wasm32-unknown-unknown)
│                          # o los emite con wasm_encoder; embebe con include_bytes!
└── src/
    ├── lib.rs             # expone InternalsManifest + los bytes WASM + firma de cada fn
    ├── abi.rs             # ABI interno: __intr_<area>_<op>; tabla de firmas
    ├── arrays.rs          # implementación WASM de arr_* (fn #[no_mangle])
    ├── strings.rs         # str_*
    ├── records.rs         # record_*
    ├── math.rs            # sqrt/floor/... (instrucciones) + minimax sin/cos/tan
    ├── convert.rs         # parse_int/float/bool, str_*
    └── layout.rs          # consts de layout compartidos (debe MATCHEAR cls-core)
```

**Detalles clave**:

1. **Un solo módulo WASM** (`internals.wasm`): todas las funciones compiladas
   juntas, exportadas con el prefijo `__intr_`. El build lo produce con
   `cargo build --target wasm32-unknown-unknown` (build.rs) o emitiéndolo con
   `wasm_encoder` (sin dependencia de target). Se embebe en el binario con
   `include_bytes!`.
2. **ABI uniforme** (igual al actual): todo por valor i64/f64/i32; strings
   `(ptr<<32)|len`; arrays `[cap][len][elems]`; records `[cap][len][(k,v,tag)*24]`.
   La memoria la comparte el módulo CLS.
3. **Forma de integración** (decidir en Fase 3):
   - (a) **Fusión de módulos**: el emisor inyecta las secciones de
     `internals.wasm` (tipos/funciones) dentro del módulo CLS antes de
     finalizar — las funciones internas viven en el mismo módulo y comparten
     memoria lineal (cero imports).
   - (b) **Import de módulo interno**: el módulo CLS importa de `"internals"`
     (un segundo módulo WASM instanciado por el motor con la misma memoria).
     Más simple, pero deja un call entre módulos (sigue WASM puro, ~ns).
   - Recomendación: **(a) fusión** — máxima velocidad y un solo módulo.
4. **Los cuerpos vienen de `cls-jit/src/host.rs`** (58 KB de lógica ya escrita
   en Rust): se portan a Rust para wasm32 (sin std I/O). El trabajo es
   mecánico en su mayoría; los tests existentes (`cls-jit/tests/host_call.rs`,
   suites) son el contrato.
5. **Canal `env.host_call`** (intrinsics del NODO, p.ej. bindings): se mantiene
   como host call — es la frontera legítima de extensión del usuario.

## Resultado esperado (de los datos de `REPORTE-BENCHMARKS.md`)

| Prueba | Hoy | Con internals WASM | Estimación vs C++ |
|---|---|---|---|
| arith (0 cruces ya) | 90.7ms | 90ms (sin cambio) | = |
| fib (fn_enter/exit + calls) | 4 391ms | 150–300ms* | x70–140 |
| arrays (push inline) | 345ms | 20–40ms | x100–200 |
| strings (concat inline) | 52ms | 3–8ms | x50–100 |
| math (sqrt nativa) | 80.3ms | 15–25ms | x8–15 |

\* El fib depende también del shadow call stack (fn_enter/fn_exit → Fase 3) —
ambos fixes van juntos para el máximo impacto.

## Criterios de aceptación

- [ ] `internals.wasm` compila y se embebe; tamaño reportado (<50 KB ideal).
- [ ] Las funciones internas pasan los mismos casos que los hosts actuales
  (portar los tests de `cls-jit/tests/` + un test de paridad host vs wasm).
- [ ] 0 imports `env.arr_*/env.str_*/env.record_*/env.math_*` en el WAT emitido
  para los scripts de benchmark (verificar con `CLS_DUMP_WAT=1`).
- [ ] Benchmark `runner-bench.ps1` antes/después documentado en el commit.

## Riesgos y decisiones pendientes

1. **Precisión de `sin/cos/tan/log/pow`**: minimax inline (precisión ~1e-12,
   suficiente para juegos/gráficos) vs mantenerlos como host call (libm del SO).
   **Decisión necesaria** — propuesta: minimax para sin/cos, `pow` por
   exp/log, `log` minimax; documentar la precisión.
2. **`json.parse/stringify`**: parseo determinista → candidato a WASM; si el
   riesgo es alto se deja como host en esta fase.
3. **Hash de records en WASM**: el hash actual del host debe replicarse
   (Fnv1a probablemente) — verificar paridad de orden de keys.
4. **wasmi (browser)**: los internals son WASM puro → funcionan igual en wasmi.
   Verificar que el tamaño del módulo no rompa el playground.
5. **Prelude del REPL**: el REPL compila por línea — la fusión de internals debe
   soportar sesiones (el módulo interno se re-inyecta por línea; medir costo).
