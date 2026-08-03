# Arquitectura del lenguaje

## Tres capas

CLS está dividido en tres capas que separan el lenguaje del entorno:

1. **cls-core** — el lenguaje en sí. Frontend (lexer, parser, AST) y middleware
   (verificador de tipos, resolución de nombres, optimizador) y configuración.
   Es agnóstico al entorno: no sabe de archivos, red ni del sistema operativo.

2. **cls-runtime** — la ejecución. El intérprete (tree-walker), el sistema de
   valores, la biblioteca estándar core (`math`, `json`, `async`), el VFS
   (sistema de archivos virtual) y los reportes de error. Tampoco sabe del
   entorno concreto: los módulos de sistema (`fs`, `http`) y los resolvers los
   provee el nodo.

3. **Nodo** — el punto de entrada del usuario final. Hay dos nodos:
   - `clx` — el CLI de desarrollo (run, check, build, repl, lsp, maptype).
   - `clxr` — el runtime ligero para ejecutar aplicaciones empaquetadas.

El nodo es quien decide cómo conseguir los módulos (resolvers), qué módulos
internos expone (`fs`, `http`, `Lib`) y dónde imprime (consola, archivo, JSON).

## Pipeline de ejecución

```
.clsx  →  Lexer  →  Parser  →  AST  →  Tree-walker (Intérprete)
```

1. El **lexer** convierte el texto en una secuencia de tokens.
2. El **parser** construye el árbol de sintaxis abstracta (AST).
3. El **tree-walker** recorre el AST y lo ejecuta directamente.

Además, el pipeline de compilación (parcialmente planeado) añade un compilador
que produce `.clbin` (WASM) y empaquetados `.clsapp`/`.clslib`.

## Extensiones de archivo

| Extensión | Contenido |
|-----------|-----------|
| `.clsx` | Código fuente. |
| `.clsapp` | Aplicación empaquetada (zip con el código y los módulos). |
| `.clslib` | Librería compilada (zip; planeado). |
| `.clbin` | Binario compilado WASM (planeado). |
| `cls.json` | Manifiesto del proyecto. |
| `.clsi` | Interfaz de tipos (para type maps y documentación). |

## Crates y directorios

```
cls-core/src/
├── frontend/     # token, lexer, parser, ast
├── middleware/   # typeck (verificador de tipos), resolver (nombres), types, optimizer
├── config/       # manifest (cls.json), types (configs)
├── backend/      # json dump, wasm (planeado)
├── error/        # ClsError, Span, Diagnostic
└── ansi/         # colores ANSI centralizados

cls-runtime/src/
├── interpreter.rs    # el tree-walker
├── value.rs          # el sistema de valores
├── environment.rs    # scopes y variables
├── resolver.rs       # ModuleResolver (mecanismo; los nodos lo configuran)
├── error_report.rs   # formateo de errores (Plain/Console/Html/Json)
├── stdlib/           # math, json, async, primitive (métodos por tipo)
├── vfs/              # VFS (protocolos res://, app://, etc.)
├── clslib.rs         # índice de librerías .clslib
├── modules.rs        # ModuleManager
├── gc.rs             # recolector de basura (stub)
└── ffi.rs / host_api.rs / sandbox.rs  # interfaces planeadas

nodos/clx/src/        # el CLI de desarrollo
nodos/clxr/src/       # el runtime ligero
```

## Sistema de módulos (dos sistemas ortogonales)

- **Sistema A — módulos fuente**: `import "mod"`. Código `.clsx` o stdlib.
  Resuelto por el `ModuleResolver` (configurable por nodo). En el build, los
  módulos se serializan como AST y se empaquetan dentro del `.clsapp`.
- **Sistema B — librerías compiladas**: `Lib.load("./lib.clslib")`. El
  `.clslib` es un zip que contiene `.clbin` (WASM). Resuelto por el
  `ClsLibResolver` (separado, configurable por nodo). Equivale a un `.dll`/`.so`.
  Va junto al `.clsapp`, no dentro.

## Separación de responsabilidades

- El **core/runtime** cargan y ejecutan módulos, recolectan exports
  (`Interpreter::load_module_source`) y verifican tipos. Nunca saben de dónde
  vienen los módulos ni qué internos existen.
- El **nodo** provee los resolvers (cómo conseguir un módulo: archivo, registry,
  red) e inyecta los internos del nodo (`fs`, `http`, `Lib`). El core/runtime no
  conocen `fs`/`http`/`Lib`.
