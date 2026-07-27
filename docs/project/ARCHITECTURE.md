# Arquitectura

CLS sigue una arquitectura de 3 capas:

```
Código fuente (.clsx)
    │
    ▼
┌───────────────┐
│   FRONTEND    │  Lexer → Parser → AST
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  MIDDLEWARE   │  Type Checker, Name Resolver, Optimizer
└───────┬───────┘
        │
        ▼
┌───────────────┐
│   BACKEND     │  Tree-walker (ahora), JIT/WASM (futuro)
└───────────────┘
```

## Flujo de una app CLS

```
DESARROLLO:
  app.clsx → cls-core → AST → cls-runtime → ejecución

PRODUCCIÓN (nodo Rust):
  .clsapp → cls-runtime (crate) → ejecución

PRODUCCIÓN (otro lenguaje):
  .clsapp → cls-runtime (.wasm) → ejecución
```

## ¿Qué es un "nodo"?

Un nodo es el ejecutable/binario que corre CLS en un entorno específico.
Cada nodo configura:

- **Intrinsics**: funciones top-level (print, input, etc.)
- **Resolver**: qué módulos están disponibles y cómo cargarlos
- **Stdlib**: módulos que el entorno provee (fs, http, etc.)

El nodo `clx` es el nodo desktop estándar que incluye acceso a filesystem y red.
