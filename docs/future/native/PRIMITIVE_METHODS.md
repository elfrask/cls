# Métodos de tipos primitivos (sin boxing) — diseño para compilación nativa

## Problema

CLS necesita métodos sobre tipos de datos primitivos (`"hola".upper()`,
`[1,2].length`, `(1,2).join(",")`), pero el lenguaje está orientado a
compilación nativa/WASM. Envolver primitivos en objetos (boxing) contradice el
modelo de `docs/future/native/NATIVE_AOT.md` (tipos planos `i64`/`f64`/`ptr`).

## Solución actual (tree-walker)

Los métodos se resuelven por **dispatch tables estáticas** en
`cls-runtime/src/stdlib/primitive.rs`:

```
PrimitiveType → { nombre → Method(NativeFn) | Getter(NativeFn) }
```

- El `Value` primitivo permanece plano. El receiver viaja como `args[0]`.
- `resolve_primitive_method()` (interpreter.rs) enlaza el receiver a un
  `FunValue` bound (`__method__.<nombre>`) para llamadas, o lo ejecuta
  directamente para getters.
- Mutadores de `Array` devuelven el array mutado y `evaluate_call` hace
  **write-back** automático a la variable (si el receiver es un `Identifier` y
  el resultado es `Array`).

## Proyección a compilador nativo/WASM

El tipo de un primitivo se conoce en **compile-time** (el checker lo resuelve).
Por tanto:

1. **Monomorfización**: `s.upper()` donde `s: String` → dirección directa de
   `string_upper`, sin lookup ni indirección. La "tabla" es solo la definición
   del AST; el compilador la resuelve estáticamente.

2. **Sin representación adicional**: el receiver es un valor plano en registro/
   stack (`i64`, `f64`, `ptr` a un string de arena/region). No hay `Value`
   wrapper, no hay vtable en el dato.

3. **Getters como funciones**: `"hola".length` → `string_len(ptr)` directo.

4. **Semántica de valores**: strings/numbers son inmutables (nuevo valor por
   transformación); arrays mutables con write-back explícito (el compilador
   emite `store` sobre la variable tras la llamada al mutador).

## Regla de oro

> Nunca convertir un primitivo en un objeto. Los métodos viven en el tipo,
> no en el dato.

Esto mantiene la representación compacta de `Value` (o de los tipos planos
nativos) y habilita la monomorfización que exige un binario eficiente.
