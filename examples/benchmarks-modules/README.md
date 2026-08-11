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
    ├── main.clsx          (entry — importa los 4 módulos)
    └── lib/
        ├── fib.clsx       (recursión — fib(26))
        ├── primos.clsx    (bucles + % — primos hasta 10000)
        ├── collatz.clsx   (bucles + aritmética — secuencia más larga hasta 5000)
        └── arrays.clsx    (push + suma de array — 5000 elementos)
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
- **JIT/WASM**: ⚠️ **limitación conocida** con módulos importados — el type map
  usa `Span { línea, col }` **sin el archivo** (cls-core/src/error/diagnostic.rs),
  por lo que las coordenadas de un módulo y de `main` pueden colisionar. El
  backend busca el tipo de una expresión del módulo con el span del main (o
  viceversa) → "Expresión sin tipo concreto". Afecta a cualquier módulo importado
  que tenga bucles/variables (no solo recursión).

### Resolución pendiente (JIT multi-módulo)

Para que el JIT compile módulos importados de forma fiable hace falta que el type
map distinga por archivo. Opciones:

1. **Agregar `file: String` al `Span`** (o un id) y usarlo como clave en
   `types_by_span`, `func_types`, etc. — cambio grande pero correcto.
2. **Desplazar los spans** de cada módulo del prelude con un offset de línea
   grande (p.ej. `+100000 * idx_modulo`) al fusionarlo, y que el backend use el
   mismo offset al buscar — requiere re-mapear el AST del módulo.

Documentado en `agent-context/JIT_VS_WALKER.md` (gap: multi-módulo en JIT).
