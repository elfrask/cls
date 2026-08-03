# Instalación

## Requisitos

- Rust estable (edition 2021). Versión mínima recomendada: 1.70 o superior.
- `cargo` y `rustc` disponibles en el `PATH`.
- (Opcional) un compilador C y `make` si quieres usar las herramientas de build
  de los scripts.

## Compilar el proyecto

El workspace contiene cuatro crates:

- `cls-core` — frontend (lexer, parser, AST), middleware (type checker,
  resolución de nombres, optimizador) y configuración.
- `cls-runtime` — el intérprete (tree-walker), el sistema de valores, la
  biblioteca estándar core, el VFS y los reportes de error.
- `nodos/clx` — el CLI de desarrollo (`clx`).
- `nodos/clxr` — el runtime ligero (`clxr`), capaz de ejecutar archivos
  empaquetados `.clsapp`.

Para compilar todo:

```
cargo build --workspace
```

Los binarios quedan en `target/debug/`:

- `target/debug/clx.exe` — el CLI de desarrollo.
- `target/debug/clxr.exe` — el runtime ligero.

Para una compilación optimizada:

```
cargo build --release
```

## Scripts de ayuda

El directorio `scripts/` incluye envoltorios para Windows y Unix:

- `clx.cmd` / `clx.sh` — ejecuta `clx`.
- `clxr.cmd` / `clxr.sh` — ejecuta `clxr`.
- `clx-build.cmd` / `clx-build.sh` — compila `clx`.
- `clxr-build.cmd` / `clxr-build.sh` — compila `clxr`.

## Verificar la instalación

```
clx --help
```

Debe mostrar la lista de subcomandos disponibles (ver `guia/cli.md`).

## Extensiones de archivo

- `.clsx` — código fuente.
- `.clsapp` — aplicación empaquetada (zip con el código y los módulos).
- `.clslib` — librería compilada (zip; planeado, requiere el backend WASM).
- `.clbin` — binario compilado WASM (planeado).
- `cls.json` — manifiesto de proyecto.
- `.clsi` — interfaz de tipos (para type maps y documentación).

## Dependencias externas

La implementación actual no requiere dependencias del sistema para el runtime
core. Los módulos `fs` y `http` del nodo desktop usan el sistema de archivos y
la red del SO a través de la biblioteca estándar de Rust.
