# Compilación Nativa AOT (Backend LLVM)

## Objetivo

Compilar CLS a **binarios nativos** (`.exe`/`.so`/`.a`) vía LLVM, alcanzando
**90-100% del rendimiento de C** — el nivel de Rust/C++.

> **Nota**: este es el camino de MÁXIMO rendimiento. CLS hoy mantiene el enfoque
> WASM-first. Este documento preserva el diseño para cuando el frontend y el
> backend WASM estén maduros.

## Modelo multi-target (como Rust)

```
.clsx → Frontend (lexer/parser/typeck/AST)
              ├─→ Backend WASM    → .clbin   (portable)
              └─→ Backend LLVM    → .exe/.so (nativo, máximo rendimiento)
```

```
clx build --target wasm   app.clsx    → app.clbin
clx build --target native app.clsx    → app.exe / libapp.so
```

Rust hace esto (`x86_64`, `arm`, `wasm32` con el mismo frontend). CLS debería igual.

## Pipeline nativo

1. **AST tipado** → IR lineal (instrucciones 3-address, MIR-style)
2. IR → **LLVM IR** (via `inkwell`/`llvm-sys`)
3. LLVM optimiza (register alloc, inlining, SIMD) → **binario nativo**

```
.clsx → Parser → TypeChecker → AST tipado → IR → LLVM IR → LLVM opt → binario
```

## Qué desbloquea el rendimiento nativo

| Decisión | Intérprete hoy | Compilado rápido |
|----------|----------------|------------------|
| `Value` | enum con `Box` | `i64`/`f64`/`ptr` sin boxear |
| GC | `Arc<Mutex<Environment>>` | Arena/region, o GC fuera del hot path |
| Magic methods | lookup string | **resolución compile-time → llamada directa** |
| `Any`/dinámico | todo es `Any` | restringido a slow path |
| `structure` | `Vec<Value>` | **struct LLVM plano (offsets fijos)** |
| Variables | `HashMap<String, Value>` | **slots indexados** en stack/registros |

## Mapeo de tipos

| CLS | LLVM |
|-----|------|
| `int` | `i64` |
| `float` | `f64` |
| `bool` | `i1` |
| `String` | `{ i8*, i64 }` (ptr + len) |
| `Array<T>` | `{ T*, i64, i64 }` (ptr, len, cap) |
| `Tuple<...>` | `{ ... }` plano (sin mutadores, sin puntero extra) |
| `structure` | `struct { ... }` (layout plano) |
| `fn(params) -> ret` | función LLVM |

## Estructuras: layout plano vs punteros

Las `structure` NO usan genéricos ni inferencia (por diseño, para que el layout
sea predecible y se compile a lo que es: un struct C).

- **Campos de tipo plano** (`Int`, `Float`, `Bool`, `Char`, `String`, otra
  `structure`, `Tuple`): van **embebidos** en el struct (offsets fijos).
- **Campos de tipo complejo** (una clase `Object`, un `Array`, un `Record`,
  una `Promise`): van como **punteros** a heap/arena (como C/Rust
  `struct Foo { x: i32, obj: Box<dyn Trait> }`). La integridad de memoria se
  preserva: el campo es una referencia, no el dato embebido.

```rust
// CLS:  structure Persona { var id: Int; var pet: Mascota; }
// LLVM: %Persona = type { i64, %Mascota* }   // Mascota* = puntero
```

Con `--no-gc` + arena/region, los punteros de campos complejos apuntan a la
región; los planos no requieren gestión.

## Magic methods en nativo

- `__add__`, `__equals__`, `__toString__`, `__get__`, `__set__`, `__next__`, ...
- Se resuelven en **compile-time**: cada clase genera una tabla de despacho
- `a + b` donde `a: Person` → `Person__add(a, b)` directo (no lookup)
- El compilador **inlinea** si el método es pequeño

## GC en modo nativo

| Modo | Cuándo | Descripción |
|------|--------|-------------|
| `--gc runtime` | General | GC preciso embebido, similar a Java |
| `--no-gc` | Embebidos/hot paths | Solo `structure` y tipos sin heap; arena/region |
| Arena | Alta perf | Asignaciones en región, liberación por lote |

Para **embebidos y binarios puros**: `structure` + `--no-gc` + arena = memoria mínima,
sin collector, layout plano.

## FFI nativo

- Exports CLS → símbolos C (`#[no_mangle]`-style)
- Imports: llamar C/Rust desde CLS
- Ver [`FFI.md`](FFI.md)

## Rendimiento esperado

| Config | Velocidad rel. a C |
|--------|---------------------|
| LLVM AOT, tipos estáticos, sin boxear | 90-100% |
| + structure plano | 95-100% (igual C struct) |
| + SIMD (`--target native` con `-C target-cpu=native`) | 100%+ en loops numéricos |
| Con GC runtime (no `--no-gc`) | 85-95% |

## Dependencias

```toml
# cls-core/Cargo.toml
inkwell = { version = "0.4", features = ["llvm18-0"] }
```

## Timeline estimado

- IR intermedio: 3-6 meses
- Backend LLVM básico (aritmética, funciones, control flow): 6-12 meses
- Types, structure, magic methods compile-time: 6-9 meses
- GC/arena, FFI: 6-9 meses
- **Total para un compilador nativo usable: 1.5-3 años**

## Prioridad de implementación

1. Subconjunto estático: `int`, `float`, `bool`, `String`, control flow, funciones
2. `structure` (layout plano) — clave para memoria y velocidad
3. Magic methods `__` resueltos en compile-time
4. Arrays compactos, sin clonado
5. GC/arena
6. FFI nativo
7. SIMD (`--target native`)
