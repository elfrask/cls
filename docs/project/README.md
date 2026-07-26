# CLS 2.0 — Proyecto

CLS es un lenguaje de programación modular, multiplataforma y portable.
Está diseñado para ser usado como lenguaje de scripting, sistema de plugins,
y como lenguaje de propósito general.

## Filosofía

- **Todo es WASM**: el compilador, el runtime y las apps se compilan a WebAssembly.
- **Modular**: el core es mínimo. Los nodos extienden la funcionalidad.
- **Host-agnóstico**: CLS puede ejecutarse en cualquier entorno que soporte WASM.
- **Embebible**: puede ser usado como librería desde Rust, Python, JS, Go, C# y más.

## Componentes

| Componente | Descripción |
|-----------|-------------|
| `cls-core` | Compilador: lexer, parser, type checker, backend |
| `cls-runtime` | Motor de ejecución: intérprete tree-walker, stdlib core |
| `ccls` | CLI principal: `run`, `check`, `build`, `ast` |
| `ccls-repl` | REPL interactivo (pendiente) |
| `cpkg` | Gestor de paquetes y proyectos (pendiente) |
| `ecls` | Ejecutor directo de `.clsapp` (pendiente) |

## Estructura del proyecto

```
cls/
├── cls-core/          # Compilador (librería)
├── cls-runtime/       # Motor de ejecución (librería)
├── nodos/
│   ├── ccls/          # CLI principal
│   ├── ccls-repl/     # REPL interactivo
│   ├── cpkg/          # Gestor de paquetes
│   └── ecls/          # Ejecutor de apps
├── scripts/           # Scripts de ejecución y build
├── docs/              # Documentación
└── host-libs/         # Wrappers para otros lenguajes
```

## Requisitos

- Rust 1.80+
- Cargo
- (Opcional) wasm32-unknown-unknown target para compilación WASM

## Compilación

```bash
# Compilar todo
cargo build

# Solo el CLI
cargo build --bin ccls

# Release
cargo build --release --bin ccls

# Ejecutar
cargo run --bin ccls -- run archivo.ccls
# o directamente
.\scripts\ccls.cmd run archivo.ccls     # Windows
./scripts/ccls.sh run archivo.ccls      # Linux/macOS
```
