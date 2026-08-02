# FFI e Interoperabilidad Nativa

## Objetivo

Permitir que CLS interactúe con **código nativo** (C, C++, Rust) y viceversa.
Esto desbloquea: linkeo de librerías nativas, carga de `.clslib` desde apps nativas,
y comunicación bidireccional entre ecosistemas.

> **Nota**: enfoque actual es WASM-first. Este documento preserva el diseño para
> el futuro nativo, y parte de él (imports WASM) ya aplica al runtime WASM.

## Direcciones de interoperabilidad

```
┌────────────┐     FFI      ┌──────────────┐
│  CLS app   │ ◄──────────► │  C/Rust lib  │
└────────────┘              └──────────────┘

┌────────────┐     FFI      ┌──────────────┐
│  Native app│ ◄──────────► │  CLS lib     │  (.clslib)
└────────────┘              └──────────────┘
```

## 1. CLS → C/Rust (importar funciones nativas)

```clx
# Declarar una función nativa (sin cuerpo)
function glClearColor(r: float, g: float, b: float, a: float) -> void;

# Llamarla
glClearColor(0.0, 0.0, 0.5, 1.0);
```

### En WASM (hoy aplicable)

- CLS compila a `.clbin` con **imports** WASM
- El host (app nativa, clxr) provee esas funciones:
  ```rust
  linker.func_wrap("env", "glClearColor", |r: f64, g: f64, b: f64, a: f64| { ... })
  ```

### En nativo (futuro)

- Funciones sin cuerpo → `extern "C"` (linkado en build time)
- `clx build --target native -l opengl32` (flags de linker)
- Symbols: `#[no_mangle]`-style exports

## 2. C/Rust → CLS (cargar librerías CLS desde nativo)

```rust
// Rust host cargando una librería CLS compilada
let lib = cls_load("math.clslib");           // .clslib = zip con .clbin + manifest
let result = lib.call("pow", &[2.0, 8.0]);   // llamar función CLS
```

### En WASM (aplicable)

- `.clslib` = zip con `.clbin`
- Host WASM instancia el módulo y llama exports:
  ```rust
  let instance = linker.instantiate(&mut store, &module)?;
  let pow = instance.get_typed_func::<(f64, f64), f64>(&mut store, "pow")?;
  let r = pow.call(&mut store, (2.0, 8.0))?;
  ```

### En nativo (futuro)

- `.clslib` nativa = `.so`/`.dll` con símbolos C exportados
- `dlopen`/`LoadLibrary` en runtime, o linkeo en build time
- ABI C estable: funciones CLS exportadas como `extern "C"`

## 3. Comunicación bidireccional

### App nativa ↔ librería CLS

```
Native app ── llama ──► CLS lib (.clslib)
    ▲                        │
    └── callback ◄───────────┘   (CLS llama de vuelta a native)
```

- CLS registra callbacks que el host nativo invoca
- El host inyecta funciones que CLS importa
- Datos se pasan como valores planos (i64/f64/ptr+len)

### App CLS ↔ librerías nativas

```
CLS app (.clsapp/.clbin) ── import ──► native lib (.so/.dll)
```

- CLS declara funciones sin cuerpo → el host las provee
- Tipos: escalares directos; strings/arrays como (ptr, len)

## Modelo de datos a través de FFI

| Tipo CLS | ABI |
|----------|-----|
| `int` | `i64` |
| `float` | `f64` |
| `bool` | `i32` (0/1) |
| `String` | `(char*, usize)` — no poseído, copia si es necesario |
| `Array<T>` | `(T*, usize, usize)` |
| `structure` | puntero a struct plano (C layout) |
| `fn` | puntero a función |

## ABI estable

- Necesario: una **ABI C estable** para que las librerías compiladas de CLS sean
  compatibles entre versiones
- Modelo: `#[repr(C)]` sobre `structure`, exports `extern "C"`

## .clslib (formato final)

```
.clslib (zip)
├── module.wasm     # o libnativa.so/.dll
├── manifest.json   # name, version, exports (nombres + firmas)
└── types.json      # type map para tooling (autocompletado)
```

### Resolución (ya diseñado)

- `Lib.load("name")` busca:
  1. `./libs/{name}.clslib`
  2. `~/.cls/clslibs/names/{name}.clslib`
  3. `~/.cls/clslibs/index.json` → by-hash/
- `ClsLibResolver` trait: cada nodo provee el I/O (core/runtime no tocan el FS)

## Seguridad

- WASM: sandbox natural (memoria lineal aislada, imports explícitos)
- Nativo: sin sandbox — el FFI nativo es tan seguro como el código que linkeas
- Para app hosting código no confiable: WASM siempre

## Timeline

| Etapa | Qué | Plazo |
|-------|-----|-------|
| 1 | Imports WASM en `.clbin` (host provee funciones) | con el backend WASM |
| 2 | Cargar `.clslib` WASM desde Rust host | post-WASM |
| 3 | FFI nativo (imports C, exports C) | post-nativo |
| 4 | Carga dinámica `.clslib` nativa (`dlopen`) | post-nativo |

## Dependencias candidatas

```toml
# host Rust que carga CLS
wasmtime = "16"       # runtime WASM (Cranelift)
libloading = "0.8"    # dlopen/LoadLibrary para .clslib nativa
```
