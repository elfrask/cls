# Módulos del nodo desktop

Los módulos desktop existen **solo en el nodo `clx`** (implementación en
`nodos/clx/src/modules/`). `clxr` no los incluye: su resolver expone únicamente
la stdlib core.

Están disponibles en `clx run` con el JIT (host functions `env.*` del nodo).
Las firmas canónicas viven en `cls-runtime/clsi/` (`fs.clsi`, `http.clsi`,
`strings.clsi`).

> Los módulos `os`, `path`, `process`, `time`, `random` se **eliminaron en
> dev-2** (no tenían consumidores; sus capacidades se cubren con `when (os:)`
> + `extension` — ver `lenguaje/multi-entorno.md` y `lenguaje/extension.md`).

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

## `strings` - utilidades de strings (internals fusionadas)

| Función | Firma | Descripción |
|---|---|---|
| `indexOf` | `indexOf(s: String, sub: String) -> int` | Índice de `sub` en `s` (`-1` si no está). |
| `slice` | `slice(s: String, start: int, end: int) -> String` | Substring `[start, end)`. |
| `split` | `split(s: String, sep: String) -> Array` | Divide por separador (array de `String`). |

Implementadas como internals fusionadas (`__intr_str_*`): 0 cruces
WASM→host.

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