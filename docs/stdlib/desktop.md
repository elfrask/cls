# Módulos del nodo desktop

Los módulos desktop existen **solo en el nodo `clx`** (implementación en
`nodos/clx/src/modules/`). `clxr` no los incluye: su resolver expone únicamente
el core (`math`, `json`, `async`).

Están disponibles en `clx run` tanto con el JIT (host functions `env.*` del
nodo) como con el walker. Las firmas canónicas viven en `cls-runtime/clsi/`
(`fs.clsi`, `http.clsi`, `os.clsi`, `path.clsi`, `process.clsi`, `time.clsi`,
`random.clsi`, `Lib.clsi`).

## `fs` - sistema de archivos

```clx
import "fs" as fs;

fs.writeFile("a.txt", "hola");
print(fs.readFile("a.txt"));
```

| Función | Firma | Descripción |
|---|---|---|
| `readFile` | `readFile(path: String) -> String` | Lee un archivo como texto. |
| `writeFile` | `writeFile(path: String, content: String)` | Escribe contenido a un archivo. |
| `exists` | `exists(path: String) -> bool` | ¿Existe el path? |
| `rm` | `rm(path: String)` | Elimina archivo (o directorio, recursivo). |
| `mkdir` | `mkdir(path: String)` | Crea directorios (recursivo). |
| `listDir` | `listDir(path: String) -> Array` | Nombres de entradas del directorio (array de `String`). |
| `cwd` | `cwd() -> String` | Directorio de trabajo actual. |

Los paths con protocolo (`app://`, `user://`, `tmp://`) se resuelven contra el
VFS del nodo (ver `runtime/vfs.md`); los demás van al filesystem real.

## `http` - cliente HTTP

| Función | Firma | Descripción |
|---|---|---|
| `get` | `get(url: String) -> String` | GET; devuelve el body de la respuesta. |
| `post` | `post(url: String, body: String) -> String` | POST; devuelve el body de la respuesta. |

## `os` - sistema operativo

| Función | Firma | Descripción |
|---|---|---|
| `platform` | `platform() -> String` | Nombre del SO (`windows`/`linux`/`macos`/...). |
| `arch` | `arch() -> String` | Arquitectura de la CPU (`x86_64`, `aarch64`, ...). |
| `version` | `version() -> String` | Versión del sistema operativo. |
| `hostname` | `hostname() -> String` | Nombre del host. |
| `home` | `home() -> String` | Directorio home del usuario. |
| `tempdir` | `tempdir() -> String` | Directorio temporal del sistema. |
| `cpus` | `cpus() -> int` | Número de núcleos de CPU disponibles. |
| `pid` | `pid() -> int` | PID del proceso actual. |
| `uptime` | `uptime() -> int` | Segundos desde el boot (**0 si no disponible**; la implementación actual siempre devuelve 0). |
| `env` | `env(key: String) -> String` | Variable de entorno (`""` si no existe). |
| `sep` | `sep() -> String` | Separador de rutas del sistema. |
| `isWindows` | `isWindows() -> bool` | ¿El sistema es Windows? |
| `isUnix` | `isUnix() -> bool` | ¿El sistema es Unix? |

## `path` - rutas de archivo

| Función | Firma | Descripción |
|---|---|---|
| `join` | `join(a: String, b: String) -> String` | Une dos segmentos de ruta. |
| `basename` | `basename(path: String) -> String` | Último componente de la ruta. |
| `dirname` | `dirname(path: String) -> String` | Directorio padre (`"."` si no tiene). |
| `extname` | `extname(path: String) -> String` | Extensión con punto (`.txt`; `""` si no tiene). |
| `resolve` | `resolve(path: String) -> String` | Ruta absoluta (relativa -> unida al cwd). |
| `normalize` | `normalize(path: String) -> String` | Normaliza `.` y `..` sin tocar el filesystem. |
| `isAbsolute` | `isAbsolute(path: String) -> bool` | ¿La ruta es absoluta? |
| `sep` | `sep() -> String` | Separador de rutas del sistema. |

## `process` - proceso actual

| Función | Firma | Descripción |
|---|---|---|
| `args` | `args() -> Array` | Args de la aplicación (los de después de `--` al invocar `clx run`). |
| `cwd` | `cwd() -> String` | Directorio de trabajo actual. |
| `env` | `env(key: String) -> String` | Variable de entorno (`""` si no existe). |
| `exit` | `exit(code: int)` | Termina el proceso con un código de salida. |
| `pid` | `pid() -> int` | PID del proceso actual. |
| `platform` | `platform() -> String` | Nombre del sistema operativo. |
| `title` | `title() -> String` | Título del proceso (`""` si no disponible; la implementación actual siempre devuelve `""`). |

## `time` - fecha y hora (UTC)

| Función | Firma | Descripción |
|---|---|---|
| `now` | `now() -> int` | Milisegundos desde la epoch Unix. |
| `seconds` | `seconds() -> int` | Segundos desde la epoch Unix. |
| `iso` | `iso() -> String` | Fecha y hora ISO 8601 (UTC, `YYYY-MM-DDTHH:MM:SSZ`). |
| `date` | `date() -> String` | Fecha UTC (`YYYY-MM-DD`). |
| `clock` | `clock() -> String` | Hora UTC (`HH:MM:SS`). |
| `year` | `year() -> int` | Año UTC. |
| `month` | `month() -> int` | Mes UTC (1-12). |
| `day` | `day() -> int` | Día UTC (1-31). |
| `hour` | `hour() -> int` | Hora UTC (0-23). |
| `minute` | `minute() -> int` | Minuto UTC (0-59). |
| `second` | `second() -> int` | Segundo UTC (0-59). |
| `sleep` | `sleep(ms: int)` | Duerme el hilo actual `ms` milisegundos. |

## `random` - aleatoriedad

| Función | Firma | Descripción |
|---|---|---|
| `random` | `random() -> float` | Aleatorio en `[0, 1)`. |
| `int` | `int(min: int, max: int) -> int` | Entero aleatorio en `[min, max]` (**inclusivo**). |
| `float` | `float(min: float, max: float) -> float` | Float aleatorio en `[min, max)`. |
| `uuid` | `uuid() -> String` | UUID v4. |

## `Lib` - librerías compiladas

```clx
import "Lib" as Lib;

Lib.load("util");   # String con el source de la librería
```

| Función | Firma | Descripción |
|---|---|---|
| `load` | `load(name: String) -> String` | Devuelve el source de una librería `.clslib` como `String`. **Stub**: no la ejecuta. |

`load` es un stub: si el contenido del `.clslib` no es texto UTF-8 válido
devuelve un marcador `<binary N bytes>` (en la implementación actual incluye el
nombre de la librería en lugar de `N`). No ejecuta el binario.

### Búsqueda de `.clslib`

Orden del resolver desktop (`DesktopLibResolver`):

1. **Path directo** - si `name` contiene `/`, `\` o termina en `.clslib`, se
   lee tal cual (VFS o filesystem).
2. `./libs/{name}.clslib` - librería local del proyecto.
3. `~/.cls/clslibs/names/{name}.clslib` (home = `HOME`/`USERPROFILE`).
4. Índice `~/.cls/clslibs/index.json` -> entrada por nombre -> archivo
   `~/.cls/clslibs/by-hash/{hash}/{name}.clslib`.

Si no se encuentra: error `Lib.load: '<name>' no encontrado`.