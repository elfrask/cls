# Backend WASM

Documentos de diseño del backend WASM (`.clbin`).

| Documento | Contenido |
|-----------|-----------|
| `WASM_PIPELINE.md` | Cómo se compila CLS a WASM: pipeline, IR, mapeo de tipos, empaquetado `.clbin`, ejecución en `clxr`. |
| `MEMORY_GC.md` | **Gestión de memoria y GC**: memoria lineal, layouts, arena/allocator, estrategias de GC (`--no-gc`, mark-sweep, generacional), raíces, plan de fases. |
| `JIT_RUNTIME.md` | Compilación en caliente con Cranelift: modelo, integración en `clxr`, rendimiento esperado. |

## Orden de lectura

1. `WASM_PIPELINE.md` — qué se produce.
2. `MEMORY_GC.md` — dónde viven los objetos y cómo se liberan.
3. `JIT_RUNTIME.md` — cómo se ejecuta y acelera.
