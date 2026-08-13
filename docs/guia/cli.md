# CLI

El binario `clx` es el CLI de desarrollo. El binario `clxr` es el runtime
ligero.

## clx

### `clx new [path]`

Crea un nuevo proyecto CLS con la estructura mínima:

- `cls.json` — manifiesto del proyecto.
- `src/main.clsx` — punto de entrada.

### `clx run <archivo.clsx> [args...]`

Compila (lexer + parser + typeck) y ejecuta un archivo con el **JIT** (CLS → WASM
→ wasmtime), el intérprete objetivo por defecto. Los argumentos adicionales se
pasan a `main(args)`.

```
clx run main.clsx
clx run app.clsx --arg1 --arg2
```

- `--jit, -j` — obsoleto (el JIT ya es el default); se acepta por compatibilidad.
- `--ast-walker` — ejecuta con el tree-walker **DEPRECADO** (solo referencia
  sintáctica; imprime una advertencia y se eliminará tras CLS 2.0-dev1).

Durante la ejecución resuelve los `import "mod"` leyendo `<mod>.clsx` del
directorio de trabajo, además de los módulos internos (`math`, `json`, `async`)
y los del nodo desktop (`fs`, `http`, `Lib`).

### `clx check [path] [--strict]`

Verifica tipos sin ejecutar. Acepta un archivo o un directorio (en cuyo caso
recorre todos los `.clsx`). Reporta errores y advertencias con contexto de
código (línea + caret).

- `--strict` activa el modo estricto: las asignaciones incompatibles son error.
- Resuelve los imports del archivo (y de los módulos importados) y registra sus
  tipos como prelude, de modo que `var c: Color` con `Color` importado sea
  verificable.

### `clx repl`

REPL interactivo. Evalúa expresiones y declaraciones de una en una. Para salir
usa `Ctrl+C` o `:salir`.

### `clx build [path]`

Prepara el proyecto para empaquetar (pipeline futuro). Genera el AST serializado
de los módulos para incluirlo en el `.clsapp`.

### `clx lsp`

Lanza el servidor de lenguaje (Language Server Protocol) sobre stdio. Lo usa la
extensión de VS Code. Ofrece autocompletado, diagnóstico y documentación basados
en los type maps.

### `clx ast <archivo.clsx>`

Muestra el árbol de sintaxis abstracta (AST) de un archivo en formato de
depuración.

### `clx maptype [path] -o <dir> [--watch]`

Genera los type maps (`.type.json`) a partir de los archivos `.clsx` y `.clsi`
de `path`, escribiéndolos en `dir`. Los type maps alimentan el autocompletado
de la extensión de VS Code.

- `--watch` / `-w` regenera automáticamente al detectar cambios (polling).

## clxr

`clxr` ejecuta aplicaciones sin las herramientas de desarrollo:

```
clxr <archivo.clsx|app.clsapp>
```

- Con un `.clsx`, compila y ejecuta el archivo directamente.
- Con un `.clsapp`, abre el paquete (zip) a través del VFS (`app://`) y ejecuta
  el punto de entrada.
- Carga los módulos internos core (`math`, `json`, `async`); no incluye los
  módulos del nodo desktop (`fs`, `http`, `Lib`).

Los errores de ejecución muestran el trazo completo numerado (call stack con
código fuente por frame), igual que `clx run`.

## Códigos de salida

- `0` — éxito.
- `1` — error de compilación, verificación o ejecución.
