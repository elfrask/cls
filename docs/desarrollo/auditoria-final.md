# Fase 4 — Auditoría final: rendimiento, capacidad y calidad

> **Estado**: PLAN. **Depende de Fases 1–3.**
> Cierre del ciclo: verificación integral de que el refactor, el crate de
> internals y la adaptación del core cumplen los objetivos de rendimiento,
> capacidad y calidad, con reportes QA completos (técnico + práctico).

## 4.1 Rendimiento (benchmarks comparativos)

**Fuente**: `examples/audit/benchmark-langs/` (CLS vs C++/Rust/Python/JS, 6
pruebas + desglose por operador) + `runner-bench.ps1`.

| Prueba | Baseline actual (REPORTE-BENCHMARKS.md) | Objetivo post-Fase 3 |
|---|---|---|
| arith (20M) | 90.7ms (= C++ 91) | mantener paridad |
| fib(30) | 4 391ms (x2091) | ≤ 300ms (x70–140) |
| arrays (100k) | 345ms (x2156) | ≤ 40ms |
| strings (10k) | 52ms (x1926) | ≤ 10ms |
| math (200k) | 80.3ms (x44) | ≤ 25ms |
| llamadas (1M) | 1 919ms | ≤ 200ms |

**Adicionales**:
- Startup del JIT (compile CLS→WASM con internals fusionados): medir que la
  inyección del módulo de internals no agregue >20ms al arranque.
- REPL: tiempo por línea con internals (el preludio por línea).
- `perf-fib(28)` de las suites internas como referencia histórica.
- wasmi (browser simulado): mismo corpus de magics/internals — paridad de
  resultados y tiempo razonable (<10x wasmtime).

**Reporte**: `agent-context/audit/general_reporte/REPORTE-RENDIMIENTO-FINAL.md`
con tabla comparativa completa y factores vs C++.

## 4.2 Capacidad (stress y límites)

Re-ejecutar la batería de stress (`examples/audit/stress/`):

| Prueba | Criterio |
|---|---|
| `stress-array-1m` | 1M pushes con internals — tiempo y memoria estables (sin corrupción tras el realloc interno) |
| `stress-array-300k/500k` | límites del allocator: `memory.grow` interno correcto |
| `stress-string-100k` | concat de 100k con el buffer amortizado — len correcto |
| `stress-recursion` / `stress-fact-100k` | shadow stack: "stack overflow" limpio, sin miles de frames |
| `stress-infinite-while` | comportamiento esperado (timeout) |
| `stress-aritmetica`, `stress-bucles-anidados` | overflow i64 documentado |
| `stress-1e300` | notación científica intacta |
| Módulos internos en REPL | sesiones largas: pool de strings + transferencia de estado con internals fusionados |

**Reporte**: `agent-context/audit/practical_qa/reports/fase4/capacidad.md`.

## 4.3 Calidad (suites, errores y paridad)

| Suite | Criterio |
|---|---|
| `run-availible.ps1` | 25 PASS |
| `run-tests.ps1` | 20 PASS + 7 SKIP esperados |
| `cargo test` (workspace) | sin fails nuevos; revisar `host_call_wasmi` (deuda pre-existente — debe quedar resuelto o documentado con plan) |
| Errores estándar (`errors.md`) | 8 casos con trace completo: sintaxis/typeck/runtime/módulos — el shadow stack debe producir IDÉNTICO formato |
| Magic methods (24/24) | re-correr `jit-magic-all.clsx` + scripts `magic-*.clsx` |
| Bindings | C (harness 13), Python (8), JS (10 + typecheck) — el motor compartido cambió |
| REPL | sesión completa: estado persistente, truthiness, magics, errores con caret |
| `clx check` | paridad de diagnostics (incluido el span de `expr_display` unificado) |
| Deprecación walker | `run --ast-walker` sigue funcionando igual (paridad de salida en las 20 pruebas de run-tests) |

**Reportes**:
- `agent-context/audit/technical_qa/reports/fase4/tecnico.md` — revisión del
  diff de las 3 fases, deuda restante, riesgos.
- `agent-context/audit/practical_qa/reports/fase4/practico.md` — outputs crudos.
- `agent-context/audit/general_reporte/REPORTE-FASE4.md` — veredicto consolidado.

## 4.4 Criterios de aprobación final

- [ ] Todos los objetivos de rendimiento (4.1) cumplidos o con desviación
  documentada y justificada.
- [ ] Cero regresiones en las suites y en los bindings.
- [ ] Errores con formato idéntico (la regla NO negociable de AGENTS.md).
- [ ] Documentación actualizada: `docs/desarrollo/*` (estas fases),
  `docs/runtime/jit.md` (nueva arquitectura de internals),
  `docs/desarrollo/arquitectura.md` (crate `cls-internals` en el workspace).
- [ ] Commit de cierre con los reportes y el CSV de benchmarks.

## 4.5 Orden de ejecución global

```
Fase 1: refactor atómico          → commit por bloque, suites verdes
Fase 2: crate cls-internals       → internals.wasm + tests de paridad host/wasm
Fase 3: adaptación del core       → emisor → internals; shadow stack; flag trace_calls
Fase 4: auditoría final           → benchmarks + stress + suites + reportes
```
