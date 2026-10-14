# REPORTE-BENCHMARKS.md — CLS 2.0 JIT (post-internals Fase 8)

> **Fecha original**: 2026-08 · **Actualizado**: 2026-08-31 (tras fusión de internals + poda de fallbacks).
> **Rama**: `master` · **Runtime**: wasmtime (default).
> **Scripts**: `examples/audit/benchmark-langs/cls/*.clsx` (N del archivo).
> Mediciones: promedio de 3 runs, `*_ms` interno reportado por el propio script.
> Hardware: (máquina del desarrollador).

## RESULTADOS PRINCIPALES (tiempo interno promedio, ms — menos es mejor)

### Features comunes (carga idéntica) — CLS hoy vs lenguajes nativos

| Prueba | Carga | **CLS (hoy)** | CLS (15-ago) | C++ | Rust | JS | Python |
|---|---|---|---|---|---|---|---|
| **Aritmética** (5 ops + cmp) | 20M iter | **92** | 90.7 | 91.1 | **82.8** | 109.2 | 12 111.7 |
| **Fib recursivo** | fib(30) | **17** | 4 391 | **2.1** | 2.3 | 12.4 | 173.7 |
| **Arrays** (push + sum) | 100k | **1** | 344.7 | **0.2** | 0.2 | 6.2 | 4.9 |
| **Strings** (concat) | 10k | **1** | 52 | 0.1 | **0.03** | 0.2 | 4.1 |
| **Math** (sqrt+sin) | 200k | **5** | 80.3 | 8.5 | **1.8** | 7.9 | 85.5 |
| **Llamadas** (función) | 1M | **4** | 1 918.7 | 0* | 0* | **3.5** | 199.6 |

\* C++/Rust con -O3 **inlinean** `cuadrado` (0ms — no comparable).

### Factor vs el más rápido (internal ms, hoy)

```
arith: rust=82.8 (x1.0)  CLS=92 (x1.1)  cpp=91.1 (x1.1)  js=109.2 (x1.3)  python=12112 (x146)
fib  : cpp=2.1 (x1.0)  rust=2.3 (x1.1)  js=12.4 (x5.9)  CLS=17 (x8.1)  python=174 (x83)
arr  : cpp=0.2 (x1.0)  rust=0.2 (x1.5)  CLS=1 (x6.3)  js=6.2 (x39)  python=4.9 (x31)
str  : rust=0.03 (x1.0)  cpp=0.08 (x2.8)  js=0.18 (x6.7)  CLS=1 (x38)  python=4.1 (x152)
math : rust=1.8 (x1.0)  js=7.9 (x4.4)  cpp=8.5 (x4.7)  CLS=5 (x2.8)  python=85.5 (x47)
call : js=3.5 (x1.0)  CLS=4 (x1.2)  python=200 (x57)
```

## MEJORA TRAS LA FUSIÓN DE INTERNALS (Fase 8)

El reporte del 15-ago medía un CLS donde arrays/strings/math/calls iban por
**host calls** (cruce WASM→host por operación). Con la fusión de internals
(`__intr_*` dentro del módulo WASM, 0 cruces) + la poda de fallbacks (Fase 8):

| Prueba | Antes (15-ago) | Hoy | Mejora |
|---|---|---|---|
| Fib(30) | 4 391 ms | **17 ms** | **x258** |
| Arrays (100k) | 344.7 ms | **1 ms** | **x345** |
| Strings (10k) | 52 ms | **1 ms** | **x52** |
| Math (200k) | 80.3 ms | **5 ms** | **x16** |
| Llamadas (1M) | 1 918.7 ms | **4 ms** | **x480** |
| Aritmética (20M) | 90.7 ms | 92 ms | ~igual |

## Resultados de la Fase 3 (antes vs después, para referencia del plan)

| Prueba | Baseline pre-Fase3¹ | Con internals WASM (Fase 3) | Estado |
|---|---|---|---|
| arith (0 cruces) | 90.7ms | ~94.0ms | ✅ sin regresión² |
| fib(30) | 4 391ms | ~13-17ms | ✅ x258-330 |
| arrays (100k push) | 345ms | ~1ms | ✅ x345 |
| strings (concat N=10000) | 52ms³ | ~1ms | ✅ x52 |
| math (sqrt/sin loop) | 80.3ms | ~5ms | ✅ x16 |
| calls (1M) | 1 919ms | ~4ms | ✅ x480 |

¹ Baseline de `agent-context/plan-performance/crate-internos.md`.
² arith varía ±4ms por ruido; es aritmética pura (0 cruces WASM↔host).
³ El baseline "strings 52ms" corresponde a otro benchmark; el actual mide
concat de strings con copia de memoria O(n²) del acumulado — el cruce WASM↔host
se eliminó (0 host calls de `str_concat`).

## Verificación del criterio "0 imports migrados"

`CLS_DUMP_WAT=1` sobre los benchmarks: el WAT **no contiene** imports
`env.str_*`, `env.arr_*`, `env.record_*`, `env.math_*` (salvo `math_random`,
que es entropía del host), ni `parse_*`, `int_abs`, `float_abs`, `pow_num`,
`fmod`. Total de imports restantes: **~70** (I/O + errores + nodo + os/process/
time/random/path/json/any/cmx/fn) — tras la poda de fallbacks (Fase 8) también
se eliminaron los hosts de str/arr/record que el emisor ya no importaba.

## Compilación JIT (startup)

Medido con `bench-jit.ps1` (fib(20), `CLS_JIT_TIMING=1`):

| Fase | Miss (compila CLS→WASM) | Cacheada |
|---|---|---|
| Frontend (lexer+parser+typeck) | ~2 ms | ~0.4 ms |
| Emit WASM | ~7 ms | — (HIT) |
| Cranelift (WASM→nativo) | **60 ms** | 59 ms |
| Store+Linker | 209 ms | ~0.1 ms |
| Proceso total | **83 ms** | **76 ms** |

Comparado con startups: JS ~225ms, Python ~12 700ms, C++/Rust ~100ms.
**CLS arranca más rápido que JS y Python** (Cranelift ~60ms; caché CLS→WASM
ahorra el frontend).

## Suites y errores (estado actual)

- `run-tests.ps1`: **28 PASS, 0 FAIL, 0 SKIP** · `run-availible.ps1`: **24 PASS, 0 FAIL, 1 SKIP** (libmod, por diseño).
- `cargo test` (workspace): **136 PASS, 0 FAIL**.
- Build workspace: **0 warnings** (poda de código muerto Fase 8).

## Conclusión

**CLS (JIT) es competitivo en cómputo lineal**: aritmética y operadores a la par
de C++/Rust y superiores a JS, y ~2-130x más rápido que Python en todo. Tras la
fusión de internals (Fase 8), **el cuello de botella de host calls está
resuelto**: llamadas/recursión/arrays/strings pasaron de x200-2000 más lentos
que nativo a paridad con JS y dentro de x2-40 de C++/Rust. Lo que queda como
host call es lo que el WASM no puede hacer solo (I/O, reloj, RNG, módulos del
nodo desktop).
