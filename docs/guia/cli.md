# CLI

El binario `clx` es el CLI de desarrollo. El binario `clxr` es el runtime
ligero (ver `herramientas/clxr.md`).

```
clx <subcomando> [opciones] [argumentos]
```

## Subcomandos

| Subcomando | Estado |
|---|---|
| `new`, `add`, `remove`/`rm`, `install`/`i`, `run`, `check`, `repl`, `build`, `lsp`, `ast`, `maptype`, `clean`, `-v`/`--version`, `-h`/`--help` | Implementados |
| `init`, `tree`, `fmt` | **Placeholder** - imprimen "no implementado" y salen con código 1 |
| `--quiet` | Silencia logs; se usa **antes** del subcomando: `clx --quiet run ...` |

Todos los subcomandos aceptan `-h`/`--help` (imprimen su ayuda y salen con 0,
sin efectos colaterales).

## `clx new <nombre> [--lib]`

Crea un proyecto nuevo con la estructura mínima:

```
<nombre>/
├── cls.json          # manifiesto (entry: "src/main.clsx")
├── .gitignore        # modules/, dist/, .cls-types
├── modules/          # dependencias instaladas
└── src/
    └── main.clsx     # function main(args: String[]) -> int
```

- `--lib` - proyecto librería: `entry` vacío, `target: "library"` y no genera
  `main.clsx`.
- El nombre no puede empezar con `-` (los flags se interpretan como opciones);
  `clx new -h` muestra la ayuda del subcomando.

## `clx add <paquete> [--dev]` · `clx remove|rm <paquete>` · `clx install|i`

Gestión de dependencias con `cls.json`:

- `add` - agrega `"<paquete>": "^1.0.0"` a `dependencies` (o `devDependencies`
  con `--dev`). Requiere un `cls.json` en el directorio actual.
- `remove` - quita el paquete de `dependencies` o `devDependencies`.
- `install` - descarga cada dependencia desde el registry como
  `modules/<paquete>/mod.clsx` y escribe el lockfile `cls.lock`.

El registry se toma de, en orden: `CLS_REGISTRY` > `cls.json["registry"]` >
`https://registry.cls-lang.org`. Si la descarga falla, se usa el caché local.

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
- `--ast-walker` - ejecuta con el tree-walker **DEPRECADO** (imprime una
  advertencia en stderr; solo referencia sintáctica).
- `--target <tripla>, -t` - simula el entorno (`arch-os-abi`) para la
  directiva `when` (no cambia la compilación).
- `--` - separador de argumentos de la aplicación.

Los `import "mod"` se resuelven con el resolver del JIT: caché de módulos ->
directorio del archivo que importa -> `modules/` del proyecto -> módulos
globales `~/.cls/modules/` (ver `lenguaje/modulos.md`).

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

- `manifest.json` - `{name, version, entry, format: "source"}`.
- `source.clsx` - el código fuente crudo del entry.

Default de salida: `dist/app.clsapp`. Nota: el empaquetado es de **código
fuente** (el AST/WASM embebido es un trabajo futuro).

## `clx repl`

REPL interactivo con el **JIT** (WASM + wasmtime) y **estado persistente**
entre líneas (variables, arrays y strings sobreviven). Evalúa declaraciones
completas; las expresiones sueltas se imprimen. Comandos: `exit`, `quit`,
`:exit`, `:quit`, `:salir`, `:q`; ayuda con `:help`/`:h`. Ver
`herramientas/repl.md`.

## `clx lsp [--silent]`

Lanza el servidor LSP sobre **stdin/stdout**. `-s`/`--silent` suprime el banner
de inicio. Ver `herramientas/lsp.md`.

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
| `CLS_REGISTRY` | Registry para `clx install` |
| `CLS_JIT_RUNTIME=wasmi` | Usa wasmi en lugar de wasmtime (sin excepciones) |
| `CLS_DUMP_WAT` | Imprime el WAT del módulo compilado en stderr |
| `CLS_JIT_TIMING=1` | Log de tiempos por fase del JIT |
| `CLS_LIB_PATH` | Directorio del binario `clsb` para los bindings Python |

## Códigos de salida

- `0` - éxito.
- `1` - error de compilación, verificación o ejecución.
