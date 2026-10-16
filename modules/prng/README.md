# `prng` — PRNG determinista (LCG) en CLS puro

Módulo **CLS puro** (sin FFI, sin `host_calls`) con un generador congruencial
lineal de 64 bits. Reemplaza al `host_call` `math.random` por una secuencia
**reproducible** que depende solo de la semilla.

Es el mismo LCG (constantes de PCG) que usaba `examples/conformance/src/fuzz.clsx`;
ahora vive en `/modules` y la prueba lo consume en vez de redefinirlo
(`examples/patrones.md` §11.5).

## API

```clx
export class Prng {
    function main(seed: int);          # fija la semilla
    function next() -> int;            # siguiente estado (s = s*A + C)
    function nextInt(bound: int) -> int;    # [0, bound); 0 si bound <= 0
    function nextFloat() -> float;     # [0, 1)
};
```

LCG: `s_{n+1} = s_n * 6364136223846793005 + 1442695040888963407` (i64, envuelve).

Uso:

```clx
import "prng" as prng;

var p = prng::Prng(88172645463325252);
var a = p.next();
var b = p.nextInt(1000);
var f = p.nextFloat();
```

## Consumir

```clx
import "prng" as prng;
```
