# JIT en el Runtime (`clxr` con Cranelift)

> **Estado**: el JIT ya está **funcionando en `clx`** (`clx run --jit`, CLS → WASM →
> wasmtime). Este documento describe la evolución futura hacia `clxr` con
> `.clbin` (formato de distribución) y el JIT en caliente.

## Estado actual (implementado)

- El nodo `clx` tiene el **JIT** completo: `nodos/clx/src/jit.rs` compila el AST
  tipado a WASM (backend `cls-core/src/backend/wasm.rs`) y lo ejecuta en
  **wasmtime** (Cranelift).
- Uso: `clx run --jit archivo.clsx`.
- El JIT es el **intérprete objetivo** (ver `AGENTS.md`); el tree-walker es
  referencia sintáctica.
- Detalle técnico: `agent-context/JIT_COMPILATION.md`, `agent-context/JIT_VS_WALKER.md`.

## Objetivo (futuro)

Hacer que `clxr` cargue **`.clbin`** (WASM empaquetado) y lo ejecute con wasmtime,
en vez de interpretar. Esto lleva el rendimiento a **50-70% de C** — el mismo
modelo que Java usa con HotSpot sobre `.jar`.

## Modelo

```
.clbin (WASM) → clxr → Cranelift JIT → código nativo (cacheado)
                              ↑
                    hot paths recompilados a nativo
```

| Componente | Rol |
|------------|-----|
| `.clbin` | Formato de distribución portable (el `.jar`) — futuro |
| `clxr` | Runtime host que carga `.clbin` — futuro |
| **Cranelift** | Compila WASM → código de máquina en runtime (vía wasmtime) |

## ¿Por qué Cranelift?

- **Wasmtime** lo usa: maduro, compila WASM a nativo rápido
- AOT o JIT: puede compilar todo al cargar, o solo hot paths
- Integración Rust limpia
- Alternativa: `wasmer` (con Singlepass/Cranelift/LLVM backends)

## Mapeo de tipos

| CLS | WASM | Cranelift/nativo |
|-----|------|------------------|
| `int` | `i64` | registro `i64` |
| `float` | `f64` | registro `f64` |
| `bool` | `i32` (0/1) | registro |
| `String` | `i64` (ptr<<32\|len) | ptr+len |
| `Array<T>` | ptr + len + stride | puntero |
| `structure` | memoria lineal con offsets | acceso directo a memoria |

## Estrategia JIT

### Opción A: Compilar todo al cargar
- Simple: compila todo el `.clbin` a nativo al inicio
- Overhead de arranque (como Java startup)
- Bueno para: apps que corren mucho tiempo

### Opción B: Interpretar + JIT hot paths (estilo V8/LuaJIT)
- Interpreta primero, detecta funciones calientes, recompila
- Mejor startup, mejor uso en loops
- Más complejo (profiling)

**Recomendación**: empezar con **A** (simple, suficiente), luego evolucionar a B.

## Rendimiento esperado

| Config | Velocidad rel. a C |
|--------|---------------------|
| WASM interpretado | 10-20% |
| WASM + Cranelift JIT | 50-70% |
| + tipo estático + inline de magic methods | 70%+ |

El JIT actual en `clx` ya da speedups de ~8000x vs el walker en `fib(26)`
(ver `examples/benchmark/`).

## Timeline estimado (para `.clbin`/clxr)

- Integrar `wasmtime` en `clxr`: ~1-2 semanas
- Mapear intrinsics/modules CLS → imports WASM: ~2-4 semanas
- Compilar examples a `.clbin` y correrlos: iterativo
- Total para un JIT funcional: **2-4 meses**

