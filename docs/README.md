# Documentación de CLS

CLS es un lenguaje de programación con verificación de tipos (compile-time),
compilado a **WASM** y ejecutado con un **JIT** (`clx run`: CLS → WASM →
wasmtime). El proyecto es un workspace Rust con seis crates — `cls-core`,
`cls-runtime`, `cls-jit`, `nodos/clx`, `nodos/clxb`, `nodos/clxr` — y bindings
de Python (`bindings/python`, paquete `clsb`).

> **JIT = intérprete objetivo.** El tree-walker (`clx run --ast-walker`) está
> **DEPRECADO**: es solo referencia sintáctica y se eliminará tras
> CLS 2.0-dev1. Toda ejecución normal usa el JIT.

## Estructura de la documentación

### Guías de uso

- `guia/instalacion.md` — requisitos, compilación desde fuente, scripts.
- `guia/inicio-rapido.md` — primer programa y sintaxis esencial.
- `guia/cli.md` — todos los subcomandos de `clx` y `clxr`.
- `guia/configuracion.md` — `cls.json`, variables de entorno, caché.

### El lenguaje

- `lenguaje/sintaxis.md` — léxico: literales, strings, comentarios, keywords, operadores.
- `lenguaje/tipos.md` — el sistema de tipos compile-time: tuplas, uniones, alias, interfaces, extracción, genéricos.
- `lenguaje/datos.md` — arrays, tuplas, records, strings e interpolación en runtime.
- `lenguaje/control-de-flujo.md` — `if`, `while`, `loop`, `for`, `for each`, `switch`, `with`, `break`, `continue`.
- `lenguaje/funciones.md` — funciones, parámetros con default, arrow functions, closures, `main`.
- `lenguaje/oop.md` — clases, herencia, visibilidad, `super`, `is`, `me`, magic methods.
- `lenguaje/enums.md` — enums con identidad, comparación e iteración.
- `lenguaje/estructuras.md` — `structure` (datos planos).
- `lenguaje/modulos.md` — `import` / `from import` / `include`, exports, resolución.
- `lenguaje/errores.md` — `throw`, `try/catch/finally` y errores en runtime.
- `lenguaje/multi-entorno.md` — directiva `when` (implementaciones por SO/arquitectura).
- `lenguaje/extension.md` — FFI a librerías nativas del sistema (`extension`).
- `lenguaje/cmx.md` — el lenguaje de marcado CMX (JSX-like).

### Biblioteca estándar

- `stdlib/core.md` — intrinsics globales (`print`, `input`, `toString`, `int`, ...) y módulos core `math`, `json`, `async`.
- `stdlib/desktop.md` — módulos del nodo desktop: `fs`, `http`, `os`, `path`, `process`, `time`, `random`, `Lib`.
- `stdlib/primitivos.md` — métodos de tipos primitivos (sin boxing).

### Runtime y ejecución

- `runtime/jit.md` — el JIT: pipeline CLS → WASM → wasmtime, caché, host functions, límites.
- `runtime/walker.md` — el tree-walker DEPRECADO (referencia sintáctica).
- `runtime/errores.md` — el sistema de errores y sus formatos.
- `runtime/vfs.md` — VFS (sistema de archivos virtual), protocolos y `.clsapp`.

### Herramientas

- `herramientas/repl.md` — REPL interactivo.
- `herramientas/lsp.md` — servidor LSP (autocompletado, diagnósticos, hover).
- `herramientas/maptype.md` — generador de type maps (`.type.json`).
- `herramientas/clxr.md` — runtime ligero para ejecutar apps.
- `herramientas/clxb.md` — bindings C para embedding (`clsb.h`).
- `herramientas/python.md` — bindings de Python (paquete `clsb`).

### Desarrollo

- `desarrollo/arquitectura.md` — workspace, crates, pipeline del compilador.
- `desarrollo/contribuir.md` — workflow, estilo y reglas.
- `desarrollo/testing.md` — cómo ejecutar y escribir tests.
- `desarrollo/agregar-feature.md` — cómo agregar una feature al lenguaje.
- `desarrollo/agregar-modulo-interno.md` — cómo agregar un módulo interno.

## Convenciones

- Los ejemplos de código usan bloques de lenguaje `clx`.
- Esta documentación solo cubre **lo implementado y accesible hoy**. Las
  features futuras se planifican en `agent-context/` (fuera de `docs/`); si un
  documento menciona algo que no existe en el código, es un error de
  documentación.
- Comentarios en CLS usan `#` hasta el final de línea (no existe `//`).
- Las rutas internas se escriben como texto (p. ej. `cls-core/src/...`).
