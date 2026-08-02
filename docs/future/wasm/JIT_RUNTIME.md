# JIT en el Runtime (`clxr` con Cranelift)

## Objetivo

Hacer que `clxr` **compile WASM a código nativo en caliente** (JIT), en vez de interpretar.
Esto lleva el rendimiento de ~2-10% a **50-70% de C** — el mismo modelo que Java usa con
HotSpot sobre `.jar`.

## Modelo

```
.clbin (WASM) → clxr → Cranelift JIT → código nativo (cacheado)
                              ↑
                    hot paths recompilados a nativo
```

| Componente | Rol |
|------------|-----|
| `.clbin` | Formato de distribución portable (el `.jar`) |
| `clxr` | Runtime host que carga `.clbin` |
| **Cranelift** | Compila WASM → código de máquina en runtime |

## ¿Por qué Cranelift?

- **Wasmtime** lo usa: maduro, compila WASM a nativo rápido
- AOT o JIT: puede compilar todo al cargar, o solo hot paths
- Integración Rust limpia
- Alternativa: `wasmer` (con Singlepass/Cranelift/LLVM backends)

## Integración en `clxr`

```rust
// clxr/src/main.rs (futuro)
use wasmtime::{Engine, Module, Store, Linker};

fn run_clbin(path: &str) -> Result<i32> {
    let engine = Engine::default();                    // Cranelift
    let module = Module::from_file(&engine, path)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Exponer funciones host a CLS (print, fs, etc.)
    linker.func_wrap("env", "print", |s: &mut Store<()>, v: i64| {
        // print desde CLS
    })?;

    let instance = linker.instantiate(&mut store, &module)?;
    let main = instance.get_typed_func::<(), i32>(&mut store, "main")?;
    main.call(&mut store, ())
}
```

## Mapeo de tipos

| CLS | WASM | Cranelift/nativo |
|-----|------|------------------|
| `int` | `i64` | registro `i64` |
| `float` | `f64` | registro `f64` |
| `bool` | `i32` (0/1) | registro |
| `String` | `(i32, i32)` ptr+len | 2 registros |
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

## Magic methods y JIT

- `__add__`, `__equals__`, `__get__`, `__next__`...
- En WASM se compilan a **tablas de funciones**
- Cranelift las puede **inline** en hot paths
- El costo desaparece en código caliente

## Rendimiento esperado

| Config | Velocidad rel. a C |
|--------|---------------------|
| WASM interpretado | 10-20% |
| WASM + Cranelift JIT | 50-70% |
| + tipo estático + inline de magic methods | 70%+ |

## Timeline estimado

- Integrar `wasmtime` en `clxr`: ~1-2 semanas
- Mapear intrinsics/modules CLS → imports WASM: ~2-4 semanas
- Compilar examples a `.clbin` y correrlos: iterativo
- Total para un JIT funcional: **2-4 meses**

## Dependencias

```toml
# clxr/Cargo.toml
wasmtime = "16"
```
