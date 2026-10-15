# Configuración

Toda la configuración de un proyecto vive en `cls.json`
(`ModuleManifest`, ver `cls-core/src/config/manifest.rs` y
`cls-core/src/config/types.rs`). Los campos usan **camelCase**.

## Estructura de `cls.json`

| Campo | Tipo | Default | Descripción |
|---|---|---|---|
| `name` | string | - | Nombre del proyecto (obligatorio) |
| `version` | string | - | Versión semver (obligatorio) |
| `description` | string | `""` | Descripción |
| `authors` | string[] | `[]` | Autores |
| `license` | string | `"MIT"` | Licencia |
| `registry` | string | `https://registry.cls-lang.org` | Registry para dependencias |
| `entry` | string | `"src/main.clsx"` | Punto de entrada principal |
| `project` | object | - | Ver `project` |
| `compiler` | object | - | Ver `compiler` |
| `interpreter` | object | - | Ver `interpreter` |
| `dependencies` | map | `{}` | Dependencias (`"<pkg>": "^1.0.0"` o exacta) |
| `devDependencies` | map | `{}` | Dependencias de desarrollo (renombrada de `devDependencies`) |
| `testEntry` | string[] | `[]` | Archivos/globs de test para `clx test` (p.ej. `["tests/**/*.clsx"]`) |
| `pathModules` | string[] | `[]` | Rutas adicionales de búsqueda de módulos (equivale a `clx run --add-path-module`). Relativas a la raíz del proyecto. Ver `examples/patrones.md` §11 |
| `lockfileVersion` | number | ausente | Versión del lockfile |

### `project`

| Campo | Default | Descripción |
|---|---|---|
| `sourceDir` | `"src"` | Directorio de fuentes |
| `outDir` | `"dist"` | Directorio de salida |
| `target` | `"executable"` | `"executable"` o `"library"` |

### `compiler`

| Campo | Default | Descripción |
|---|---|---|
| `targetArchitecture` | `"wasm"` | `x86_64`, `arm64`, `wasm`, `bytecode` |
| `optimizationLevel` | `"O2"` | `O0`–`O3`, `Os` |
| `types` | object | Configuración de tipos (`TypesConfig`) |
| `features` | object | `async`, `macros`, `experimental` (bool, default `false`) |
| `warnings` | object | Ver abajo |
| `sourceMaps` | bool | `true` |

`types` (`TypesConfig`):

| Campo | Default | Descripción |
|---|---|---|
| `check` | `true` | Chequeo de tipos (`true` = híbrido, `false` = dinámico) |
| `strict` | `false` | Tipado estricto (solo si `check`) |
| `noImplicitAny` | `false` | Prohibir tipos no inferidos |
| `nullSafety` | `true` | Evitar null pointer exceptions |

`warnings` (`WarningsConfig`):

| Campo | Default | Descripción |
|---|---|---|
| `unusedVariables` | `"warn"` | Nivel del warning |
| `deadCode` | `"warn"` | Nivel del warning |
| `treatWarningsAsErrors` | `false` | Warnings como errores |

### `interpreter`

| Campo | Default | Descripción |
|---|---|---|
| `optimization` | `false` | Optimización en interpretación |
| `mode` | `"pure-ast"` | `"pure-ast"` \| `"jit"` |
| `runtime` | object | Memoria del runtime (`RuntimeMemoryConfig`) |
| `sandbox` | object | Sandbox (`SandboxConfig`) |

`runtime.` (`RuntimeMemoryConfig`):

| Campo | Default | Descripción |
|---|---|---|
| `memoryLimit` | `"1GB"` | Límite de memoria lineal del sandbox (bytes; `"512MB"`, `"1GiB"`, `"500"`...). **Aplicado** por el JIT (wasmtime `StoreLimits`); `"0"` = sin límite. Override: env `CLS_MEMORY_LIMIT` |
| `stackSize` | `"8MB"` | Tamaño de stack |
| `gc` | object | GC (`GcConfig`: `enabled` `false`, `strategy` `"compiled"`, `threshold` `"64MB"`) |

`sandbox.` (`SandboxConfig`):

| Campo | Default | Descripción |
|---|---|---|
| `allowFs` | `false` | Permitir acceso a filesystem (expuesto en `config()`; el módulo `fs` de CLS es futuro, ver limitaciones) |
| `allowNet` | `false` | Permitir acceso a red (expuesto en `config()`) |
| `allowProcess` | `false` | Permitir procesos/comandos (expuesto en `config()`) |
| `allowEnv` | `false` | Permitir leer variables de entorno (expuesto en `config()`) |
| `readPaths` | `[]` | Rutas de lectura permitidas |
| `writePaths` | `[]` | Rutas de escritura permitidas |
| `allowedHosts` | `[]` | Hosts de red permitidos |
| `maxExecutionTime` | `0` | Tiempo máximo de ejecución (ms; `0` = sin límite). Si es `> 0` el JIT activa la interrupción por epoch de wasmtime. Override: env `CLS_TIME_LIMIT`. **Aplicado** |
| `maxOutputBytes` | `0` | Límite de bytes de salida (`0` = sin límite). Expuesto en `config()` |
| `maxFileSize` | `0` | Límite de tamaño de archivo en bytes (`0` = sin límite). Expuesto en `config()` |

> **Sandbox de memoria**: `memoryLimit` (o `CLS_MEMORY_LIMIT`) sí se aplica —
> se declara como máximo de páginas de la memoria lineal del módulo WASM y
> además se fija como `StoreLimits` de wasmtime; al superarlo, la ejecución
> trapea con un error de ejecución (exit ≠ 0). La reserva virtual por memoria
> se reduce a 64 MiB (guard 64 KiB) para acotar el VA reservado.
>
> **Alcance**: el sandbox (`StoreLimits`, `memoryLimit`, `maxExecutionTime`) es
> **solo runtime-WASM** — limita la ejecución del programa, **no** al compilador
> ni a `clx build`. El compilador/emisión corre en el host sin esos límites.

### `config()` — introspección de la configuración (solo lectura)

El lenguaje expone la configuración efectiva del host como un **intrinsic de
introspección autorizado** (excepción sancionada a la regla de no añadir
`host_calls`; no añade features, solo **lee parámetros del host**):

```clx
from "config" import *;   # o import "config" as cfg;

config();                 // intrinsic crudo: Record<String, String> (host level)
get("project.name");      // "" si no existe
getBool("sandbox.allowNet");
getInt("sandbox.maxOutputBytes");
sandboxEnabled("net");    // -> sandbox.allowNet
```

- El intrinsic crudo `config()` devuelve un `Record<String, String>` **plano**
  (claves con puntos), construido **una sola vez** en el host y cacheado. Es
  **de solo lectura**: no expone punteros ni permite modificar la config.
- `modules/config/` (`modules/config/mod.clsx`, CLS puro) es la capa idiomática:
  `all()`, `get(key)`, `getBool(key)`, `getInt(key)`, `sandboxEnabled(name)`,
  con **caché perezosa** (una sola llamada al host).
- Claves del snapshot: `project.*`, `compiler.*`, `interpreter.*`, `sandbox.*`
  y `runtime.*` (valores efectivos: `runtime.memoryLimitBytes`,
  `runtime.timeLimitMs`, `runtime.runtime` = `wasmtime`/`wasmi`,
  `runtime.traceCalls`, `runtime.pathModules`, `runtime.target`).

## Ejemplo real

`examples/test/hello/cls.json`:

```json
{
  "authors": [],
  "dependencies": {},
  "description": "",
  "devDependencies": {},
  "entry": "src/main.clsx",
  "license": "MIT",
  "name": "hello",
  "project": {
    "outDir": "dist",
    "sourceDir": "src",
    "target": "executable"
  },
  "registry": "https://registry.cls-lang.org",
  "version": "0.1.0",
  "compiler": {
    "targetArchitecture": "arm64",
    "features": {}
  }
}
```

## Variables de entorno

| Variable | Efecto |
|---|---|
| `CLS_REGISTRY` | Registry para `clx install` (prioridad sobre `cls.json["registry"]`) |
| `CLS_GIX_DRIVER` | Ruta al driver de git (`clxgix.dll`/`.so`) para instalar del registry |
| `CLS_JIT_RUNTIME=wasmi` | Usa wasmi en lugar de wasmtime (sin excepciones CLS) |
| `CLS_DUMP_WAT` | Imprime el WAT del módulo compilado en stderr |
| `CLS_JIT_TIMING=1` | Log de tiempos por fase del JIT |
| `CLS_MEMORY_LIMIT` | Límite de memoria lineal del sandbox (p.ej. `1GB`, `512MB`); prioridad sobre `cls.json` |
| `CLS_TIME_LIMIT` | Límite de tiempo de ejecución en ms (opt-in; interrupción por epoch) |
| `CLS_LIB_PATH` | Directorio del binario `clsb` (bindings Python: `clsb.dll`/`.so`/`.dylib`) |

## Caché de compilación

- `~/.cache/cls/` - binarios WASM compilados; la clave es un hash de la
  fuente del entry + versión de `cls-core` + target + runtime + fuentes de
  todos los módulos importados. Editar cualquier `.clsx` del grafo invalida
  el caché.
- `[workspace]/.cls-cache/module-index.json` - índice de integridad
  informativo (hashes SHA-256 de cada `.clsx` del workspace; el JIT no lo
  usa para invalidar).

Limpieza:

```ps
clx clean        # vacía ~/.cache/cls (reporta archivos y bytes)
clx clean --all  # además borra el directorio completo y [cwd]/.cls-cache/
```

## Dependencias y registry

```ps
clx add <spec> [--dev]     # edita cls.json (con versión = fija; sin versión = ^última)
clx install                # resuelve deps de cls.json -> ~/.cls/modules/<name>@<v>/
clx install <ruta|zip> -S  # instala paquete local y lo registra (--save)
clx ls [-g]                # lista paquetes locales y globales
clx info [ruta|paquete]    # info de un paquete/proyecto
clx audit [-g]             # audita manifiesto/entry/deps/duplicados
```

- `<spec>` = `[scope/]nombre[@versión]` (p.ej. `foo`, `foo@1.2.3`, `usuario/foo`).
- Los paquetes instalados viven en `~/.cls/modules/[scope/]nombre@versión/`
  (layout anidado para scopes).
- `./modules/` es para los **módulos propios** del usuario.

`clx install` escribe el lockfile `cls.lock`:

```json
{
  "lockfileVersion": 1,
  "registry": "https://registry.cls-lang.org",
  "packages": {
    "foo": { "version": "1.2.3", "channel": "stable", "resolved": "global" }
  }
}
```

El registry **no almacena código**: publica `{repo_url, commit}` y `clx` lo baja
con el **driver de git** (`gix`), cargado dinámicamente (`CLS_GIX_DRIVER`).

## Resolución de `.clslib` (`ClsLibResolver`)

Para `Lib.load(...)`, el nodo desktop (`nodos/clx/src/subcommands/run.rs`)
busca en este orden:

1. Path directo (si el nombre contiene `/`, `\` o termina en `.clslib`).
2. `./libs/{name}.clslib`.
3. `~/.cls/clslibs/names/{name}.clslib`.
4. `~/.cls/clslibs/index.json` -> entry por nombre -> `~/.cls/clslibs/by-hash/{hash}/{name}.clslib`.