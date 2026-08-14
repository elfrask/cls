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

### `os` — sistema y entorno (nodo desktop)

| Función | Descripción |
|---------|-------------|
| `platform()` | Nombre del SO (`windows`/`linux`/`macos`/...). |
| `arch()` | Arquitectura de la CPU (`x86_64`, `aarch64`, ...). |
| `version()` | Versión del sistema operativo. |
| `hostname()` | Nombre del host. |
| `home()` | Directorio home del usuario. |
| `tempdir()` | Directorio temporal del sistema. |
| `cpus()` | Núcleos de CPU disponibles. |
| `pid()` | PID del proceso actual. |
| `uptime()` | Segundos desde el boot (`0` si no disponible). |
| `env(clave)` | Variable de entorno (`""` si no existe). |
| `sep()` | Separador de rutas del sistema. |
| `isWindows()` / `isUnix()` | ¿Sistema Windows / Unix? |

### `path` — rutas de archivos (nodo desktop)

| Función | Descripción |
|---------|-------------|
| `join(a, b)` | Une dos segmentos de ruta. |
| `basename(ruta)` | Último componente. |
| `dirname(ruta)` | Directorio padre (`"."` si ninguno). |
| `extname(ruta)` | Extensión con punto (`.txt`; `""` si no). |
| `resolve(ruta)` | Ruta absoluta (relativa → unida al cwd). |
| `normalize(ruta)` | Normaliza `.`/`..` sin tocar el FS (acepta `/` y `\`). |
| `isAbsolute(ruta)` | ¿La ruta es absoluta? |
| `sep()` | Separador de rutas. |

### `process` — proceso actual (nodo desktop)

| Función | Descripción |
|---------|-------------|
| `args()` | Args de la aplicación (los de después de `--`). |
| `cwd()` | Directorio de trabajo actual. |
| `env(clave)` | Variable de entorno (`""` si no existe). |
| `exit(código)` | Termina el proceso. |
| `pid()` | PID del proceso. |
| `platform()` | Nombre del SO. |
| `title()` | Título del proceso (`""` si no disponible). |

### `time` — fechas y hora (nodo desktop, UTC)

| Función | Descripción |
|---------|-------------|
| `now()` | Milisegundos desde la epoch Unix. |
| `seconds()` | Segundos desde la epoch Unix. |
| `iso()` | Fecha/hora ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`). |
| `date()` | Fecha UTC (`YYYY-MM-DD`). |
| `clock()` | Hora UTC (`HH:MM:SS`). |
| `year()` / `month()` / `day()` | Año / mes 1-12 / día 1-31 (UTC). |
| `hour()` / `minute()` / `second()` | Hora 0-23 / minuto 0-59 / segundo 0-59 (UTC). |
| `sleep(ms)` | Duerme el hilo actual. |

### `random` — aleatoriedad (nodo desktop)

| Función | Descripción |
|---------|-------------|
| `random()` | Float en `[0, 1)`. |
| `int(min, max)` | Entero en `[min, max]` (inclusivo). |
| `float(min, max)` | Float en `[min, max)`. |
| `uuid()` | UUID v4. |

> Los módulos del nodo desktop (`fs`, `http`, `Lib`, `os`, `path`, `process`,
> `time`, `random`) se usan por nombre directo en el JIT o con
> `import "mod" as mod` (ambos intérpretes). Las firmas oficiales viven en
> `cls-runtime/clsi/*.clsi`.

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
