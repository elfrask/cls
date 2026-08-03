# Biblioteca estándar

La biblioteca estándar de CLS se divide en módulos core (siempre disponibles) y
módulos del nodo (dependen del entorno).

## Módulos core

### `math`

Matemáticas. Se accede con `math.nombre` (o `import "math"`).

| Función | Descripción |
|---------|-------------|
| `abs(x)` | Valor absoluto. |
| `sqrt(x)` | Raíz cuadrada. |
| `pow(x, y)` | Potencia. |
| `min(a, b)` / `max(a, b)` | Mínimo/máximo. |
| `floor(x)` / `ceil(x)` / `round(x)` | Redondeo. |
| `random()` | Aleatorio. |
| `sin(x)` / `cos(x)` / `tan(x)` | Trigonométricas. |
| `log(x)` | Logaritmo natural. |
| `range(inicio, fin)` | Rango. |
| `PI` / `E` | Constantes. |

### `json`

Serialización JSON. Se accede con `json.nombre`.

| Función | Descripción |
|---------|-------------|
| `parse(texto)` | Analiza un texto JSON en un valor. |
| `stringify(valor)` | Convierte un valor a texto JSON. Si el objeto define `__toJson`, se usa ese resultado. |

### `async`

Utilidades asíncronas (corrutinas).

| Función | Descripción |
|---------|-------------|
| `delay(ms)` | Devuelve una promesa que se resuelve tras `ms` milisegundos. |
| `all(lista)` | Espera todas las promesas. |
| `race(lista)` | Espera la primera que se resuelva. |

Se usa con `async function` y `await` (ver `lenguaje/funciones.md`).

### `primitive`

No es un módulo importable: son los métodos de los tipos primitivos, resueltos
por tablas de despacho. Ver `runtime/metodos-primitivos.md`.

## Módulos del nodo desktop

Estos módulos interactúan con el sistema operativo y los provee el nodo `clx`.
No están disponibles en `clxr` (el runtime ligero).

### `fs`

| Función | Descripción |
|---------|-------------|
| `readFile(ruta)` | Lee un archivo como cadena. |
| `writeFile(ruta, contenido)` | Escribe un archivo. |
| `exists(ruta)` | ¿Existe? |
| `rm(ruta)` | Elimina. |
| `mkdir(ruta)` | Crea directorio. |
| `listDir(ruta)` | Lista directorio. |
| `cwd()` | Directorio de trabajo. |

### `http`

| Función | Descripción |
|---------|-------------|
| `get(url)` | Petición GET. |
| `post(url, cuerpo)` | Petición POST. |

### `Lib`

| Función | Descripción |
|---------|-------------|
| `load(ruta)` | Carga una librería `.clslib` (planeado completo). |

## Intrinsics globales

Funciones disponibles sin import:

| Función | Descripción |
|---------|-------------|
| `print(...)` | Imprime los argumentos separados por espacio. Usa `__toString`. |
| `input()` | Lee una línea de la entrada estándar. |
| `toString(x)` | Convierte a cadena. Usa `__toString`. |
| `int(x)` / `float(x)` / `bool(x)` | Conversiones de tipo. Usan `__int`/`__float`/`__bool`. |
| `str(x)` | Alias de `toString`. |
| `len(x)` | Longitud de arrays, tuplas, records, strings u objetos `__len`. |
| `type(x)` | Nombre del tipo. Usa `__type` si existe. |
| `now()` | Timestamp en milisegundos. |
| `exit(code)` | Finaliza la ejecución. |
| `sleep(ms)` | Duerme la ejecución. |
| `throw(msg)` | Lanza un error intencional. |
| `args` | Los argumentos de la línea de comandos (array de cadenas). |

## Métodos de tipos primitivos

Los tipos base tienen métodos resueltos por tablas de despacho; ver
`runtime/metodos-primitivos.md`.
