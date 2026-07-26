# CLS 2.0 — Arquitectura

CLS es un lenguaje de programación multiplataforma con un ecosistema modular basado en WASM.
El compilador, el runtime y las apps son módulos WASM intercambiables.
Cualquier lenguaje que pueda ejecutar WASM puede alojar CLS.

---

## Principios

1. **Todo es WASM** — Compilador, runtime y apps compiladas a `.wasm`
2. **Host-agnóstico** — Python, JS, Go, C#, Rust pueden alojar CLS
3. **3 Capas** — Frontend (parseo) → Middleware (análisis) → Backend (salida)
4. **Modular** — Nodos ejecutables + host libraries
5. **Embeddable** — El runtime se compila a WASM y se importa como librería en cualquier lenguaje

---

## Diagrama de ecosistema

```
                     ┌───────────────────────────────────┐
                     │         HOST (cualquier lenguaje)  │
                     │  Python (wasmtime-py)              │
                     │  JS (wasmtime-js / wasmer)         │
                     │  Go / C# / Rust / ...             │
                     └───────┬──────────┬───────────────┘
                             │          │
                ┌────────────┘          └────────────┐
                ▼                                     ▼
    ┌──────────────────────┐              ┌──────────────────────┐
    │  cls-core.wasm       │              │  cls-runtime.wasm    │
    │                      │              │                      │
    │  COMPILADOR          │              │  MOTOR DE EJECUCIÓN  │
    │  Frontend → Middle → │              │                      │
    │  Backend             │              │  ├─ Tree-walker      │
    │                      │              │  ├─ JIT engine       │
    │  Exporta:            │              │  ├─ GC               │
    │  ├─ compile()        │              │  ├─ Sandbox          │
    │  ├─ check()          │              │  ├─ Std Library      │
    │  ├─ astToJson()      │              │  └─ C FFI exports    │
    │  └─ config()         │              │                      │
    └──────────┬───────────┘              │  Exporta:            │
               │                          │  ├─ loadApp()        │
               │   produce                │  ├─ run()            │
               ▼                          │  ├─ callFunction()   │
    ┌──────────────────────┐              │  └─ loadModule()     │
    │  app.wasm            │              └──────────────────────┘
    │  (código CLS         │                       │
    │   compilado)         │                       │
    └──────────────────────┘                       │
                          ▲                        │
                          └──── ejecuta ───────────┘
```

---

## Pipeline de compilación

```
Código fuente (.ccls)
    │
    ▼
┌─────────────────────────────────────┐
│ FRONTEND                            │
│ ├─ Lexer   → Tokens                 │
│ ├─ Parser  → AST                    │
│ └─ AST nodes con spans              │
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│ MIDDLEWARE                          │
│ ├─ Type Checker (configurable)      │
│ ├─ Name Resolver (scopes)           │
│ └─ Optimizer (constant folding)     │
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│ BACKEND                             │
│ ├─ Tree-walker (pure-ast mode)      │
│ ├─ LLVM IR → JIT / AOT (futuro)    │
│ ├─ WASM codegen (futuro)            │
│ └─ JSON dump (transpilación)        │
└─────────────────────────────────────┘
```

---

## Ciclo de vida de una app CLS

```
DESARROLLO:
  app.ccls ──▶ cls-core ──▶ app.wasm ──▶ .clsapp (zip)

PRODUCCIÓN (Rust nativo):
  .clsapp ──▶ cls-runtime (crate nativo) ──▶ app run!

PRODUCCIÓN (cualquier lenguaje):
  .clsapp ──▶ cls-runtime (.wasm) ──▶ app run!

DISTRIBUCIÓN (plugin/addon):
  app.clsapp ──▶ Host (Python, JS, Go, Rust...) carga runtime internamente
```
