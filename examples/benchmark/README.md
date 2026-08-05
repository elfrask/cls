# Benchmark del lenguaje

Compara la velocidad de **CLS** frente a **Node.js** y **Python** usando las
mismas cargas de trabajo y los mismos parámetros en los tres scripts.

## Cómo correr

```powershell
# Los tres en secuencia
.\run-all.cmd

# O cada uno por separado
..\..\scripts\clx.cmd run benchmark.clsx
node benchmark.js
python benchmark.py
```

## Cargas de trabajo

Cinco micro-benchmarks clásicos, implementados de forma idéntica en los tres
lenguajes:

| Nombre | Qué mide | Parámetro |
|--------|----------|-----------|
| `fib` | recursión + llamadas a función | `fib(26)` |
| `primos` | bucles + aritmética (`%`) | contar primos hasta 10000 |
| `collatz` | bucles con aritmética entera | secuencia más larga hasta 5000 |
| `array` | push + suma de un array | 5000 elementos |
| `string` | concatenación de strings | 5000 concatenaciones |

Los tamaños están pensados para que CLS (intérprete tree-walker) termine en
unos 30 segundos. En Node/Python esos mismos números se resuelven en
milisegundos; esa diferencia es el punto de la comparación.

## Resultados (una corrida, en milisegundos)

| Benchmark | CLS | Node.js | Python | CLS/Node | CLS/Python |
|-----------|------|--------|--------|----------|-----------|
| fib(26) | ~16000 | 2.2 | 24 | ~7300x | ~670x |
| primos(10000) | ~1600 | 1.0 | 11 | ~1560x | ~145x |
| collatz(5000) | ~5400 | 2.2 | 44 | ~2430x | ~123x |
| array(5000) | ~6100 | 0.4 | 1.0 | ~16100x | ~5900x |
| string(5000) | ~50 | 0.1 | 1.9 | ~400x | ~27x |

Todos producen los mismos resultados (fib=121393, primos=1229, collatz mejor=3711
en 237 pasos, suma=12497500, len=5000), lo que confirma que la carga es la misma.

> Los tiempos de CLS varían entre corridas (por ejemplo, fib(26) oscila entre
> 13 y 16 s). Los multiplicadores son aproximados.

## Lectura honesta

- **CLS es un intérprete tree-walker** diseñado para expresividad, no para
  velocidad: cada operación pasa por dispatch dinámico, boxing de valores y
  entorno de ejecución. ~100-600x más lento que Python y ~400-16000x que Node
  dependiendo de la carga.
- **Node.js (V8)** gana casi todo; **Python** se queda a medio camino.
- El benchmark de `string` es el que menos separa a CLS de Python: la
  concatenación parece no ser O(n²) en CLS.
- Conclusiones prácticas: CLS brilla por su runtime amigable, módulos y
  tipado estático (`clx check --strict`), no para cómputo intensivo. Para
  eso están los otros dos.

## Archivos

```
benchmark.clsx   Implementación en CLS
benchmark.js     Implementación en Node.js
benchmark.py     Implementación en Python
run-all.cmd      Ejecuta los tres en secuencia
```
