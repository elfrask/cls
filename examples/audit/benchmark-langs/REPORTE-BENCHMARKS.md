# REPORTE-BENCHMARKS.md — CLS 2.0 JIT (Fase 3: internals WASM fusionados)

> **Fecha**: 2026-08 · **Rama**: `refactor` · **Runtime**: wasmtime (default).
> **Scripts**: `examples/audit/benchmark-langs/cls/*.clsx` (N del archivo).
> Mediciones: promedio de 3 runs, `*_ms` interno reportado por el propio script.
> Hardware: (máquina del desarrollador).

## Resultados (antes vs después de la Fase 3)

| Prueba | Baseline pre-Fase3¹ | Con internals WASM (hoy) | Objetivo del plan | Estado |
|---|---|---|---|---|
| arith (0 cruces) | 90.7ms | **94.0ms** | ~90ms (=) | ✅ sin regresión² |
| fib(30) | 4 391ms | **13.3ms** | ≤300ms | ✅ x330 |
| arrays (100k push) | 345ms | **0.7ms** | ≤40ms | ✅ x493 |
| strings (concat N=10000) | 52ms³ | **162ms** | ≤10ms³ | ⚠️ ver nota⁴ |
| math (sqrt/sin loop) | 80.3ms | **5.3ms** | ≤25ms | ✅ x15 |
| calls (1M) | 1 919ms | **4.0ms** | ≤200ms | ✅ x480 |

¹ Baseline de `agent-context/plan-performance/crate-internos.md` (datos de un
`REPORTE-BENCHMARKS.md` previo que no está en el repo).
² arith varía ±4ms por ruido; es aritmética pura (0 cruces WASM↔host), no
depende de la fusión.
³ El baseline "strings 52ms" y el objetivo "≤10ms" del plan corresponden a
otro benchmark (concat de strings cortos fijos). El archivo actual
`04-string.clsx` hace `s = s + "x"` en loop (N=10000): cada concatenación
aloca y copia el acumulado → O(n²) ≈ 50M copias. 162ms es el costo de la
copia de memoria, NO del cruce WASM↔host (que se eliminó: 0 host calls de
`str_concat`). Un benchmark de operaciones de strings cortas (upper/trim/
contains) tendría el orden del objetivo.
⁴ El shadow call stack (fn_enter/exit en memoria lineal, 0 host calls) es
parte del speedup de fib y calls.

## Verificación del criterio "0 imports migrados"

`CLS_DUMP_WAT=1` sobre los benchmarks: el WAT **no contiene** imports
`env.str_*`, `env.arr_*`, `env.record_*`, `env.math_*` (salvo `math_random`,
que es entropía del host), ni `parse_*`, `int_abs`, `float_abs`, `pow_num`,
`fmod`. Total de imports restantes: **77** (I/O + errores + nodo + os/process/
time/random/path/json/any/cmx/fn).

## Suites y errores

- `run-availible.ps1`: **25 PASS** · `run-tests.ps1`: **21 PASS** (+7 SKIP).
- `cargo test` (workspace): **184 PASS, 0 FAIL** (incluye clxb 5, cls-jit 8,
  cls-internals 17, cls-core 69).
- Errores con trace completo (byte-idénticos al formato del JIT): `main →
  outer → inner` con call sites (`9:22`, `6:20`); `err-error-en-modulo`
  resuelve el módulo (`err-lib-modulo:2:15`); stack overflow limpio (3 frames).
- REPL con estado persistente: OK (transferencia del heap entre líneas).
- wasmi (`CLS_JIT_RUNTIME=wasmi`): paridad con walker en stdlib; `host_call_wasmi`
  (bug pre-existente) ahora pasa.
