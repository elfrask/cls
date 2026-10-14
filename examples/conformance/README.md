# Suite de conformidad CLS 2.0 (differential testing vs C)

Prueba **#1** de `examples/PLAN-PRUEBAS.md` (bloqueante). Verifica el lenguaje CLS
de forma sistemática y reproducible comparando el mismo cómputo:

1. escrito en **CLS puro**, y
2. escrito en **C** (`oracle/oracle.c`, compilado a `oracle.dll`), expuesto por
   **FFI** (`extension`).

C es un **oráculo externo e independiente**: no hay tree-walker, así que la
divergencia CLS↔C es la señal de regresión.

Todo es **CLS puro + FFI**: no se usan `fs`/`http`/`json`/`strings` ni
`host_calls` nuevos, y **no se exponen punteros** al código CLS (el oráculo
recibe `CString`/`CLong`/`CDouble` y devuelve `CLong`/`CDouble`/`CString`;
nunca `CPtr`).

## Estructura

```
examples/conformance/
├── cls.json                 # { name, version, entry, pathModules }
├── run.ps1                  # orquestador (compila .dll, corre todo, resume)
├── oracle/
│   ├── oracle.c             # evaluador C de referencia (definido, sin UB)
│   └── oracle.dll           # compilado por run.ps1 (gcc -shared)
├── src/
│   ├── expr.clsx            # FFI + dispatch puro (compartido)
│   ├── main.clsx            # runner por eje (arith/float/strings/...)
│   └── fuzz.clsx            # fuzzer determinista (LCG, N=20000)
└── findings/                # repros mínimos de cada regresión + README
```

## Ejes cubiertos

| Eje | Qué compara | Casos |
|-----|-------------|-------|
| **arith/bitwise** | `+ - * / % & \| ^ << >>` y `== != < <= > >=`, unarios `-` `~`, sobre `i64` (bordes: `INT64_MIN/MAX`, 0, negativos, shift ≥ ancho, wrap/overflow, `%` con signo) | ~485 |
| **floats + mix** | `+ - * /` y comparaciones en `f64`; promoción `int->float` | ~173 |
| **strings (utf8)** | `length`, `indexOf`, `contains`, `startsWith`, `endsWith`, `slice` (longitud y contenido) con bytes multibyte UTF-8 | ~1558 |
| **control/recursion** | `while`, `for`, `if/elif/else`, `continue`/`break`, recursión (`fib`, `fact`, `collatz`, `gcd`) | ~41 |
| **closures/callable** | captura léxica, re-captura, `Array.map` con closure (llamada indirecta) | ~28 |
| **arrays/records/tuplas** | `push/unshift/reverse/indexOf`, records (`keys`, `has`, `length`), tuplas (índice, `length`) | ~13 |
| **excepciones** | `try/catch/finally` en salida normal, tras `catch`, anidado (propagación) y `return` dentro de `try` | ~4 |
| **fuzz** | 20000 expresiones enteras aleatorias `(op,a,b)` con semilla fija | 20000 |

## Cómo correr

Desde la **raíz del repo** (el `.dll` del oráculo se resuelve relativo al CWD):

```powershell
target\debug\clx.exe run examples\conformance\src\main.clsx --add-path-module=modules
target\debug\clx.exe run examples\conformance\src\fuzz.clsx --add-path-module=modules
```

O todo el ciclo (compila el `.dll`, fría/caliente, fuzz, findings, resumen):

```powershell
powershell -File examples\conformance\run.ps1
```

`run.ps1` **falla (exit ≠ 0)** si algún eje diverge, si el fuzz diverge o si la
salida cambia entre **caché fría y caliente**. El `cls.json` declara
`"pathModules": ["../../modules"]`.

## Oráculo (`oracle.c`)

Semántica fijada para que C sea un oráculo **definido** (nada de UB):

- **Enteros** `int64_t`: `+ - *` en aritmética *unsigned* reinterpretada = wrap
  de 64 bits (equivalente a `i64.add/sub/mul` del WASM). División/resto truncados
  hacia cero (WASM `div_s`/`rem_s`). Shifts enmascarados a `mod 64` (WASM). Las
  comparaciones devuelven 1/0.
- **Flotantes** `double` IEEE-754 (idéntico a `f64`).
- Funciones de strings por **bytes** (`strlen`/`memcmp`), con el mismo clamping
  de `slice` que CLS.
- Control/recursión y colecciones: réplica del algoritmo CLS devolviendo
  agregados enteros (nunca buffers).

### Nota: FFI flotante (`CDouble`)

El dispatcher FFI no pasaba argumentos `CDouble` por XMM — **bug corregido**:
ahora invoca con **libffi** y la firma real por slot (ver
`findings/02-ffi-f64-args.clsx`). La suite todavía usa `o_fcmp`/`o_mix_cmp`
(operandos y resultado como strings + `strtod`), un **workaround retirable**: se
puede volver a las firmas `CDouble` directas.

## Limitaciones conocidas del oráculo

- **Bytes crudos C→CLS**: el retorno `CString` ya es **byte-transparente**
  (corregido, ver `findings/03-cstring-return-utf8.clsx`). La suite todavía
  compara los slices multibyte por **longitud** (`o_slice_len`) — simplificable a
  comparación de contenido. Ruta del `.dll` relativa al **CWD**.
- **wasmi**: no soporta excepciones; `main.clsx` usa `try/catch`. Se corre como
  informativo, no como fallo.

## Resultado (JIT/wasmtime, Windows x64)

Ver `findings/README.md` y `examples/reportes/01-conformance.md`. Actualmente:
**2302/2302** casos coinciden con C en la suite y **20000/20000** en el fuzzer;
caché fría == caliente. Los 3 findings quedaron **resueltos**.
