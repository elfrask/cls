# CLS — Planes Futuros

Este documento describe los desarrollos planificados para CLS a mediano y largo plazo.
El foco actual es **WASM-first**; la compilación nativa es un camino futuro documentado aquí
para no perder el diseño.

---

## Estado actual (línea base)

```
.clsx → Lexer → Parser → AST → Tree-walker (intérprete)
                            → JSON backend (AST dump)
```

- **Intérprete**: tree-walker sobre AST
- **Runtime**: `clxr` (ejecuta `.clsx` y `.clsapp`)
- **Empaquetado**: `clx build` → `.clsapp` (zip con fuente/AST)
- **Rendimiento**: ~2-10% de C (típico de tree-walker)

---

## Visión de largo plazo

```
.clsx → Lexer → Parser → TypeChecker → Optimizer → IR → Backends
                                                      ├─→ WASM (.clbin)
                                                      ├─→ JIT (Cranelift)
                                                      └─→ Nativo LLVM (.exe/.so)
```

CLS aspira a ser un lenguaje **multi-target**: WASM para portabilidad (el formato de
distribución, como `.jar`), y nativo AOT para máximo rendimiento (como `rustc`).

---

## Tabla de contenidos

| Documento | Ruta | Contenido |
|-----------|------|-----------|
| WASM-first | [`wasm/WASM_PIPELINE.md`](wasm/WASM_PIPELINE.md) | Compilación a `.clbin` (WASM) |
| JIT en runtime | [`wasm/JIT_RUNTIME.md`](wasm/JIT_RUNTIME.md) | `clxr` con Cranelift (50-70% de C) |
| Compilación nativa | [`native/NATIVE_AOT.md`](native/NATIVE_AOT.md) | Backend LLVM (90-100% de C) |
| FFI e interoperabilidad | [`native/FFI.md`](native/FFI.md) | Linkeo nativo, carga de librerías |

---

## Decisiones de diseño pendientes

### Formato de distribución
- **`.clbin`** = WASM binario (el `.jar` de CLS)
- **`.clslib`** = zip conteniendo `.clbin` + manifiesto (la librería compilada)
- **`.clsapp`** = app empaquetada (fuente/AST, o futuramente `.clbin`)

### Runtime (`clxr`)
- Hoy: interpreta source/AST
- Futuro: interpreta `.clbin` (WASM)
- Más futuro: **JIT** (Cranelift compila WASM → nativo en caliente)
- Opcional: **AOT** pre-compilado para producción

### Nativos (futuro, documentado para no perder diseño)
- Linkeo nativo de librerías
- Carga de `.clslib` desde apps nativas
- Comunicación bidireccional: app nativa ↔ librerías CLS, y viceversa
- FFI a C/C++/Rust

---

## Rendimiento esperado por etapa

| Etapa | Pipeline | Velocidad rel. a C |
|-------|----------|---------------------|
| Hoy | Tree-walker | 2-10% |
| Paso 1 | Bytecode VM (WASM) interpretado | 10-20% |
| Paso 2 | `clxr` + JIT (Cranelift) | 50-70% |
| Paso 3 | `clx build --native` (LLVM AOT) | 70-90% |
| Meta | LLVM AOT + tipado estático + layout plano | 90-100% |

---

## Casos de uso y requisitos por dominio

| Dominio | Requisito | Etapa mínima |
|---------|-----------|--------------|
| Juegos 2D / UI | FPS estable | JIT (Paso 2) o AOT |
| Juegos 3D pesados | SIMD, GPU | Nativo AOT (Paso 3) |
| Redes neuronales | Matrices optimizadas, SIMD | Nativo AOT + SIMD |
| Big data | Representaciones compactas, sin clonado | Nativo AOT |
| Embebidos | Sin GC, memoria mínima, arranque instantáneo | Nativo AOT `--no-gc` |
| Scripting / apps rápidas | Velocidad de desarrollo | Intérprete (hoy) ✅ |
| Portabilidad total | Correr en cualquier host | WASM + `clxr` ✅ |

---

## Principios que guían el diseño futuro

1. **El frontend es reutilizable**: lexer, parser, typechecker y AST ya existen — alimentan
   todos los backends (WASM, JIT, nativo).
2. **Magic methods `__`**: diseñados para resolución en compile-time (despacho de tabla),
   no lookup de strings en hot paths.
3. **`structure`** = layout plano (como C struct) — la clave para memoria compacta y AOT.
4. **Tipos estáticos alimentan codegen**: `int` → `i64` sin boxear, no `Value::Int`.
5. **GC configurable**: en modo compilado, arena/region o GC preciso fuera del hot path.
6. **WASM es el formato, no el techo**: el JIT y el AOT son los que dan velocidad.
