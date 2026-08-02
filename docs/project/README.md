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
| `cls-runtime` | Motor de ejecución: intérprete tree-walker, stdlib core, VFS, async |
| `clx` | CLI principal: `run`, `check`, `build`, `ast`, `maptype`, `lsp` |
| `clxr` | Ejecutor directo de `.clsx` / `.clsapp` |

## Estructura del proyecto

```
cls/
├── cls-core/          # Compilador (librería)
├── cls-runtime/       # Motor de ejecución (librería)
├── nodos/
│   ├── clx/           # CLI principal
│   └── clxr/          # Ejecutor de apps
├── scripts/           # Scripts de ejecución y build
├── docs/              # Documentación
│   └── future/        # Planes futuros (WASM, JIT, nativo, FFI)
├── examples/tests/    # Scripts de ejemplo y prueba
└── .vscode/           # Extension VS Code + config

## Requisitos

- Rust 1.80+
- Cargo
- (Opcional) wasm32-unknown-unknown target para compilación WASM

## Compilación

```bash
# Compilar todo
cargo build

# Solo el CLI
cargo build --bin clx

# Release
cargo build --release --bin clx

# Ejecutar
cargo run --bin clx -- run archivo.clsx
# o directamente
.\scripts\clx.cmd run archivo.clsx     # Windows
./scripts/clx.sh run archivo.clsx      # Linux/macOS
```

## Planes futuros

Ver [`docs/future/README.md`](../future/README.md) para la visión de largo plazo:
compilación WASM (`.clbin`), runtime con JIT (Cranelift), compilación nativa (LLVM AOT)
y FFI / interoperabilidad nativa. El enfoque actual es **WASM-first**; el resto está
documentado para preservar el diseño.
