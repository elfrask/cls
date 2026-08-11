# Ejemplos JIT-only

Ejemplos que se validan **exclusivamente con el intérprete JIT** (`clx run --jit`).
El tree-walker NO se usa aquí: es solo referencia sintáctica y se deprecará tras
CLS 2.0-dev1 (ver DIRECTIVA en `AGENTS.md`).

## Cómo correr

```powershell
powershell -File examples/jit-examples/run-jit.ps1
# o directamente:
clx run --jit examples/jit-examples/modules/src/main.clsx
```

## `modules/` — imports con múltiples módulos

Proyecto CLS que importa 3 módulos con los 3 estilos de Sistema A:

| Estilo | Sintaxis | Qué demuestra |
|--------|----------|----------------|
| namespaced | `import "lib/mathx" as mathx` → `mathx::cuadrado()` | acceso `x::f` y `x::var` |
| from/alias | `from "lib/stringsx" import gritar as gritarFn, repetir` | renombrado con `as` |
| include | `include "lib/colores"` → `Color.Azul` | todos los exports inline (enums) |

```
modules/
├── cls.json
└── src/
    ├── main.clsx          (entry)
    └── lib/
        ├── mathx.clsx     (export function/var → namespaced)
        ├── stringsx.clsx  (export function → from/alias)
        └── colores.clsx   (export enum → include)
```

Los imports se resuelven **relativos al archivo que importa** (dir de `src/`),
priorizando el directorio local, luego `modules/` del proyecto y luego los
globales `~/.cls/modules/` (ver `agent-context/wasm-plan/RESOLVERS.md`).
