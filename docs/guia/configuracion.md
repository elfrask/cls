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
| `dependencies` | map | `{}` | Dependencias (`"<pkg>": "^1.0.0"`) |
| `devDependencies` | map | `{}` | Dependencias de desarrollo (renombrada de `devDependencies`) |
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
| `memoryLimit` | `"512MB"` | Límite de memoria |
| `stackSize` | `"8MB"` | Tamaño de stack |
| `gc` | object | GC (`GcConfig`: `enabled` `false`, `strategy` `"compiled"`, `threshold` `"64MB"`) |

`sandbox.` (`SandboxConfig`):

| Campo | Default | Descripción |
|---|---|---|
| `allowFs` | `false` | Permitir acceso a filesystem |
| `allowNet` | `false` | Permitir acceso a red |
| `maxExecutionTime` | `5000` | Tiempo máximo de ejecución (ms) |

## Ejemplo real

`examples/hello/cls.json`:

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
| `CLS_JIT_RUNTIME=wasmi` | Usa wasmi en lugar de wasmtime (sin excepciones CLS) |
| `CLS_DUMP_WAT` | Imprime el WAT del módulo compilado en stderr |
| `CLS_JIT_TIMING=1` | Log de tiempos por fase del JIT |
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
clx add <paquete>        # agrega "<paquete>": "^1.0.0" a dependencies
clx add <paquete> --dev  # idem, en devDependencies
clx install              # descarga a modules/<paquete>/mod.clsx
```

`clx install` escribe el lockfile `cls.lock`:

```json
{
  "lockfileVersion": 1,
  "registry": "https://registry.cls-lang.org",
  "packages": { "<paquete>": { "version": "latest" } }
}
```

## Resolución de `.clslib` (`ClsLibResolver`)

Para `Lib.load(...)`, el nodo desktop (`nodos/clx/src/subcommands/run.rs`)
busca en este orden:

1. Path directo (si el nombre contiene `/`, `\` o termina en `.clslib`).
2. `./libs/{name}.clslib`.
3. `~/.cls/clslibs/names/{name}.clslib`.
4. `~/.cls/clslibs/index.json` -> entry por nombre -> `~/.cls/clslibs/by-hash/{hash}/{name}.clslib`.