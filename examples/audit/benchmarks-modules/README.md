# Benchmarks CLS con múltiples módulos

Benchmarks de rendimiento de **CLS** usando el **sistema de módulos** (imports).
Cada benchmark vive en un módulo aparte en `src/lib/`, y `src/main.clsx` los
importa con `from "lib/..." import ...`.

## Estructura

```
examples/benchmarks-modules/
├── cls.json               (proyecto CLS)
├── run-bench.ps1          (corre walker y JIT, compara)
└── src/
    ├── main.clsx          (entry - importa los 4 módulos)
    └── lib/
        ├── fib.clsx       (recursión - fib(26))
        ├── primos.clsx    (bucles + % - primos hasta 10000)
        ├── collatz.clsx   (bucles + aritmética - secuencia más larga hasta 5000)
        └── arrays.clsx    (push + suma de array - 5000 elementos)
```

## Cómo correr

```powershell
powershell -File examples/benchmarks-modules/run-bench.ps1
```

## Cargas de trabajo (idénticas a examples/benchmark)

| Nombre | Qué mide | Resultado esperado |
|--------|----------|--------------------|
| `fib(26)` | recursión + llamadas | 121393 |
| `primos(10000)` | bucles + aritmética | 1229 |
| `collatz(5000)` | bucles con aritmética | 3711 |
| `array(5000)` | push + suma de array | 12497500 |

## Estado

- **Tree-walker**: ✅ funciona con los 4 módulos; resultados correctos.
- **JIT/WASM**: ✅ **módulos importados funcionan** (incl. loops y recursión). El
  type map usaba `Span { línea, col }` sin archivo, lo que colisionaba entre
  módulos y main; se resolvió **desplazando los spans** de cada módulo importado
  con un offset de línea único (`cls-core/src/frontend/span_shift.rs` + `run_jit`).
  Nota: en los módulos, los loops deben declarar la variable (`for (var i = ...)`)
  - la asignación implícita sin `var` no se soporta en el typeck estricto del JIT.
