# CLI

El binario `clx` es el CLI de desarrollo. El binario `clxr` es el runtime
ligero (ver `herramientas/clxr.md`).

```
clx <subcomando> [opciones] [argumentos]
```

## Subcomandos

| Subcomando | Estado |
|---|---|
| `new`, `add`, `remove`/`rm`, `install`/`i`, `ls`, `info`, `audit`, `test`, `start`, `run`, `check`, `repl`, `build`, `lsp`, `ast`, `maptype`, `clean`, `-v`/`--version`, `-h`/`--help` | Implementados |
| `login`, `publish`, `yank`, `search` | **Fase B/C** (requieren el registry) |
| `init`, `tree`, `fmt` | **Placeholder** - imprimen "no implementado" y salen con código 1 |
| `--quiet` | Silencia logs; se usa **antes** del subcomando: `clx --quiet run ...` |

Todos los subcomandos aceptan `-h`/`--help` (imprimen su ayuda y salen con 0,
sin efectos colaterales).

## `clx new <nombre> [opciones]`

Crea un proyecto nuevo con la estructura base:

```
<nombre>/
├── cls.json          # manifiesto completo (proyecto, compilador, runtime, sandbox)
├── .gitignore        # modules/, dist/, .cls-types, .cls-cache
├── modules/          # MÓDULOS PROPIOS del usuario (dev local)
└── src/
    └── main.clsx     # function main(args: String[]) -> int
```

`cls.json` incluye **todos los parámetros de proyecto, compilador, runtime y
sandbox** (incluidos `pathModules`, `pathModulesNatives` y `testEntry`, aunque
estén vacíos). `export`, `setup` y `lockfileVersion` son específicos de módulos,
paquetes y lockfiles; se agregan a mano cuando corresponden.

| Opción | Efecto |
|---|---|
| `--lib` | Librería: `target: "library"`, `entry` vacío y sin `main.clsx`. |
| `--entry <ruta>` | Entry point (default `<source-dir>/main.clsx`). |
| `--target <tipo>` | `executable` o `library`. |
| `--source-dir <dir>` | Directorio de fuentes (default `src`). |
| `--out-dir <dir>` | Directorio de salida (default `dist`). |
| `--test-entry <glob>` | Patrón de tests (repetible). |
| `--path-module <dir>` | Ruta extra de módulos (repetible). |
| `--path-module-native <dir>` | Ruta de librerías nativas (repetible). |
| `--version <semver>` | Versión del proyecto (default `0.1.0`). |
| `--description <texto>` | Descripción. |
| `--author <nombre>` | Autor (repetible). |
| `--license <id>` | Licencia (default `MIT`). |
| `--registry <url>` | Registry de dependencias. |
| `--strict` / `--no-strict` | Tipado estricto. |
| `--no-check` | Desactiva el chequeo de tipos. |
| `--no-source-maps` | Desactiva source maps. |
| `--feature-async`, `--feature-macros`, `--feature-experimental` | Features del compilador. |
| `--memory-limit <size>` | Memoria del sandbox (default `1GiB`). |
| `--stack-size <size>` | Stack (default `8MB`). |
| `--max-execution-time <ms>` | Timeout de ejecución (`0` = sin límite). |
| `--max-output-bytes <n>`, `--max-file-size <n>` | Límites del sandbox. |
| `--gc`, `--gc-threshold <size>` | GC y su umbral (default `64MB`). |
| `--allow-fs`, `--allow-net`, `--allow-process`, `--allow-env` | Permisos del sandbox. |

- El nombre no puede empezar con `-`; `clx new -h` muestra la ayuda completa.
- El resto de campos avanzados de `cls.json` se pueden editar a mano (ver
  `configuracion.md`).

## Paquetes: `add`, `remove`, `install`, `ls`, `info`, `audit`

**Dónde vive cada cosa**:

- `./modules/{nombre}/` - **módulos propios** del usuario (dev local). El resolver
  los mira **primero**.
- `~/.cls/modules/[scope/]nombre@versión/` - paquetes **instalados** (global,
  versionado).
- `cls.json → dependencies` - `nombre → rango|exacta`; el resolver elige la
  versión global que cumpla el rango.
- `cls.lock` - versiones exactas resueltas (reproducibilidad).

### `clx add <spec> [--dev]`

Solo edita `cls.json` (no descarga). `<spec>` = `[scope/]nombre[@versión]`:

- con versión → **fija** (`"foo": "1.2.3"`);
- sin versión → **rango** `^<última>` (registry si hay red → global → error
  "módulo no resuelto").
- `--dev` → `devDependencies`.

### `clx remove|rm <spec>`

Quita la dependencia de `cls.json` (no toca el store global).

### `clx install [<ruta|zip|spec>...] [-S|--save] [-f|--force]`

- **Sin argumentos**: instala las deps de `cls.json` (rangos → última que cumpla)
  y escribe `cls.lock`.
- **Con rutas**: instala paquetes locales (directorio o `.zip`/`.clsapp`) en
  `~/.cls/modules/[scope/]nombre@versión/`. **Requiere `name` y `version`
  (semver) y `export.entry`.** `--save` registra la dep exacta en `cls.json`.
- **Con specs**: instala por nombre; si no está en global, lo baja del registry
  (Fase B) usando el **driver de git** (`gix`).
- `--force` sobrescribe la versión ya instalada.
- **Setup**: si el paquete declara `setup` (o tiene `setup.clsx`), se ejecuta una
  sola vez por versión tras instalar. Recibe `CLS_MODULE_DIR`,
  `CLS_PACKAGE_ROOT`, `CLS_MODULE_NAME`, `CLS_MODULE_VERSION`,
  `CLS_GLOBAL_MODULES` y `CLS_GLOBAL_BIN`. Los ejecutables de `bin/` se copian
  a `~/.cls/bin`.

### `clx ls [-l|--local] [-g|--global]`

Lista los paquetes locales (`./modules`) y globales (`~/.cls/modules`).

### `clx info [ruta|paquete]`

Info de un directorio, `cls.json`, `.zip`/`.clsapp` o paquete por nombre; sin
argumento, del proyecto actual.

### `clx audit [-l|-g]`

Audita manifiesto, `entry`/`export.entry`, dependencias no instaladas y
duplicados. Exit 1 si encuentra problemas.

### Driver de git (`gix`)

El registry **no guarda código**: publica `{repo, commit}`. `clx` obtiene el
código con un **driver aparte** que enlaza `gix` (no pesa en el binario),
cargado dinámicamente. Se busca en `CLS_GIX_DRIVER`, junto al binario, o en
`~/.cls/drivers/`. Compilarlo: `scripts/gix-driver-build.cmd|.sh` (crate
standalone en `drivers/gix-driver/`).

## `clx test [archivos|globs] [--filter <s>] [--list]`

Ejecuta los tests del proyecto. Sin argumentos usa `testEntry` de `cls.json`
(array de rutas o globs, admite `*`, `**`, `?`). Cada archivo se ejecuta con el
JIT; su exit code determina pass/fail. Aplica `pathModules` del `cls.json` del
CWD y del proyecto de cada test. Ver `desarrollo/testing.md` y
`lenguaje/testing.md`.

## `clx start [comando]`

Construye el `PATH` con los `bin/` de todos los módulos que resuelve el
proyecto: `modules/*/bin`, `pathModules`, `~/.cls/modules` y `~/.cls/bin`.
Sin comando abre una shell interactiva; con comando lanza el proceso
directamente por el SO (sin shell intermedia), salvo `.cmd`/`.bat` en Windows:

```bash
clx start
clx start tombstone migrate:status
```

## `clx run [archivo] [--] [args...]`

Compila y ejecuta con el **JIT** (CLS -> WASM -> wasmtime), el intérprete
objetivo por defecto. Los argumentos tras `--` se pasan a `main(args)` (y
quedan disponibles vía `process.args()`).

Sin archivo, usa el `entry` de `cls.json`; si no hay manifiesto, busca
`main.clsx`, `src/main.clsx`, `mod.clsx` o `src/mod.clsx`.

```
clx run main.clsx
clx run app.clsx -- a1 a2
```

Opciones:

- `--jit, -j` - obsoleto, sin efecto (el JIT ya es el default).
- `--ast-walker` - obsoleto, sin efecto (se ignora; el JIT es el único intérprete desde dev-2).
- `--target <tripla>, -t` - simula el entorno (`arch-os-abi`) para la
  directiva `when` (no cambia la compilación).
- `--add-path-module <dir>` (o `--add-path-module=<dir>`) - agrega una ruta de
  búsqueda de módulos, repetible. Equivale a declarar `pathModules` en
  `cls.json`. Ver `examples/patrones.md` §11.
- `--` - separador de argumentos de la aplicación.

Los `import "mod"` se resuelven con el resolver del JIT: directorio del archivo
que importa -> `modules/` del proyecto -> rutas de `pathModules` /
`--add-path-module` -> CWD y `modules/` del CWD -> `modules/` junto al binario
`clx` (`{dir_clx}/modules` y `{dir_clx}/../modules`) -> módulos globales
`~/.cls/modules/` (ver `lenguaje/modulos.md` y `examples/patrones.md` §11).

## `clx check [archivo|dir] [--strict]`

Verifica tipos sin ejecutar. Acepta un archivo, un directorio (recorre todos
los `.clsx` recursivamente, saltando `modules/`, `dist/`, `libs/` y ocultos)
o el `entry` del proyecto. Resuelve los imports del grafo y registra sus
exports como prelude (los tipos importados son verificables).

- `--strict` - activa el modo estricto (asignaciones incompatibles = error).
- Reporta `[ERROR|WARN|INFO] mensaje (file:line:col)` con línea + caret.
- Código de salida: 0 sin errores, 1 con errores.

## `clx build [archivo] -o <salida>`

Empaqueta la aplicación en un `.clsapp` (zip con dos entradas):

- `manifest.json` - `{name, version, entry, format: "source"}`; si el proyecto
  usa FFI, incluye `nativeLibs` con el mapa paquete → archivos.
- `source.clsx` - el código fuente crudo del entry.

Default de salida: `dist/app.clsapp`. Nota: el empaquetado es de **código
fuente** (el AST/WASM embebido es un trabajo futuro).

Si hay extensiones nativas, `clx build` copia las librerías resueltas a
`dist/native-libs/<paquete>@<version>/`, conserva sus nombres originales,
deduplica por contenido y falla ante colisiones. Cada paquete tiene su propia
carpeta: no se mezclan versiones.

## `clx repl`

REPL interactivo con el **JIT** (WASM + wasmtime) y **estado persistente**
entre líneas (variables, arrays y strings sobreviven). Evalúa declaraciones
completas; las expresiones sueltas se imprimen. Comandos: `exit`, `quit`,
`:exit`, `:quit`, `:salir`, `:q`; ayuda con `:help`/`:h`. Ver
`herramientas/repl.md`.

## `clx lsp [--silent]`

Lanza el servidor LSP sobre **stdin/stdout**. `-s`/`--silent` suprime el banner
de inicio. El servidor indexa `.clsx`/`.clsi` del workspace y expone
diagnósticos, completion, hover, go-to-definition y document symbols. Ver
`herramientas/lsp.md`.

## `clx ast <archivo> [--json]`

Muestra el AST de un archivo: dump de depuración Rust por defecto, JSON
pretty con `--json`.

## `clx maptype [path] -o <dir> [--watch]`

Genera un `.type.json` por cada `.clsx`/`.clsi` bajo `path` (default `.`),
preservando la estructura relativa; output default `./.cls-types`. Ver
`herramientas/maptype.md`.

## `clx clean [--all]`

Limpia el caché de compilación JIT en `~/.cache/cls/` (reporta archivos y
bytes eliminados). `--all` además borra el directorio completo y el caché del
workspace `[cwd]/.cls-cache/`.

## Variables de entorno

| Variable | Efecto |
|---|---|
| `CLS_REGISTRY` | Registry para `clx install`/`add` |
| `CLS_GIX_DRIVER` | Ruta al driver de git (`clxgix.dll`/`.so`) para instalar del registry |
| `CLS_JIT_RUNTIME=wasmi` | Usa wasmi en lugar de wasmtime (sin excepciones) |
| `CLS_DUMP_WAT` | Imprime el WAT del módulo compilado en stderr |
| `CLS_JIT_TIMING=1` | Log de tiempos por fase del JIT |
| `CLS_LIB_PATH` | Directorio del binario `clsb` para los bindings Python |

## Códigos de salida

- `0` - éxito.
- `1` - error de compilación, verificación o ejecución.
