# Biblioteca estándar core

La stdlib core es la que existe **siempre** (con `clx run`, `clxr` y el REPL),
tanto en el JIT como en el tree-walker. Se compone de dos partes:

- **Intrinsics globales**: funciones sin módulo, disponibles directamente.
- **Módulos core**: `math`, `json` y `async` (este último solo en el walker —
  el JIT no compila `await`).

Las firmas canónicas viven en `cls-runtime/clsi/` (`core.clsi`, `math.clsi`,
`json.clsi`, `async.clsi`) y son la fuente de verdad para el typechecker y el
LSP.

## Intrinsics globales

Registrados en el `Interpreter` (globales del nodo + `register_core_intrinsics`)
y en el typeck vía `core.clsi`. Iguales en el JIT (host functions `env.*`) y en
el walker.

| Función | Firma | Descripción |
|---|---|---|
| `print` | `print(val...)` | Imprime en consola. Une múltiples args con un espacio. |
| `input` | `input(prompt) -> String` | Lee una línea desde la entrada estándar. |
| `toString` | `toString(val: Any) -> String` | Convierte cualquier valor a string. |
| `int` | `int(val: Any) -> int` | Convierte a entero (float → truncado, string → parseo). |
| `float` | `float(val: Any) -> float` | Convierte a flotante. |
| `str` | `str(val: Any) -> String` | Alias de `toString`. |
| `bool` | `bool(val: Any) -> bool` | Verdad del valor (`is_truthy`). |
| `len` | `len(val: Any) -> int` | Longitud de arrays, strings o records. |
| `type` | `type(val: Any) -> String` | Nombre del tipo del valor. |
| `now` | `now() -> int` | Timestamp actual en milisegundos. |
| `exit` | `exit(code: int)` | Finaliza la ejecución con un código. |
| `sleep` | `sleep(ms: int)` | Duerme la ejecución. |
| `throw` | `throw(msg: Any)` | Lanza un error de runtime intencional. |

Los args de CLI no se exponen como variable global: llegan a
`main(args: String[])` (ver `herramientas/clxr.md` y `lenguaje/funciones.md`).

> **`push(arr, val)` es un stub**: existe como función global pero siempre
> falla con `"push: usa arr.push(val) en su lugar"`. Los mutadores de array
> son métodos del tipo primitivo (ver `stdlib/primitivos.md`); la variante
> global no se debe usar.

## Módulo `math`

Se importa por nombre:

```clx
import "math" as math;

math.sqrt(16);      # 4.0
math.range(1, 5);   # [1, 2, 3, 4]
math.PI;
```

Módulo `Record` con funciones nativas + dos constantes. Firmas exactas de
`math.clsi`:

| Miembro | Firma | Descripción |
|---|---|---|
| `PI` | `var PI: float` | Valor de PI. |
| `E` | `var E: float` | Constante de Euler. |
| `abs` | `abs(x: float) -> float` | Valor absoluto. |
| `sqrt` | `sqrt(x: float) -> float` | Raíz cuadrada. |
| `pow` | `pow(base: float, exp: float) -> float` | Potencia. |
| `min` | `min(a: float, b: float) -> float` | Mínimo entre dos números. |
| `max` | `max(a: float, b: float) -> float` | Máximo entre dos números. |
| `floor` | `floor(x: float) -> int` | Redondeo hacia abajo. |
| `ceil` | `ceil(x: float) -> int` | Redondeo hacia arriba. |
| `round` | `round(x: float) -> int` | Redondeo al entero más cercano. |
| `random` | `random() -> float` | Número aleatorio en `[0, 1)`. |
| `sin` | `sin(x: float) -> float` | Seno. |
| `cos` | `cos(x: float) -> float` | Coseno. |
| `tan` | `tan(x: float) -> float` | Tangente. |
| `log` | `log(x: float) -> float` | Logaritmo natural. |
| `range` | `range(start: int, end: int) -> Array` | Rango de enteros `[start, end)`. |

## Módulo `json`

```clx
import "json" as json;

var d = json.parse("{\"a\": 1}");   # Record { a: 1 }
json.stringify(d);                  # "{\"a\":1}"
```

| Función | Firma | Descripción |
|---|---|---|
| `parse` | `parse(text: String) -> Any` | Parsea un string JSON: objetos → `Record`, arrays → `Array`, además de los escalares. |
| `stringify` | `stringify(value: Any) -> String` | Convierte un valor a JSON string. |

`stringify` respeta el magic method `__toJson`: si el objeto lo define, lo usa
para la serialización; si no, la representación nativa.

## Módulo `async` (solo tree-walker)

```clx
import "async" as async;

var p = async.delay(1000);   # Promise
```

| Función | Firma | Descripción |
|---|---|---|
| `delay` | `delay(ms: int) -> Promise` | Promesa que resuelve después de `ms` milisegundos. |
| `all` | `all(promises: Array) -> Promise` | Resuelve cuando todas las promesas resolvieron (con todos los resultados). |
| `race` | `race(promises: Array) -> Promise` | Resuelve con la primera promesa que resuelve. |

> **El JIT no compila `async`**: `await` y el módulo `async` del walker no
> entran en el subconjunto WASM; el emisor aborta con error explícito. Este
> módulo solo funciona en el tree-walker (deprecado).