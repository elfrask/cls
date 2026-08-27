# Decisión 001 — Sin coerción implícita entre primitivos

**Estado**: vigente. **No negociable** para el emisor WASM/JIT y el typeck.

**Fecha**: dev-2 (post-limpieza de pendientes).

---

## Regla

No existe coerción implícita entre primitivos y objetos en CLS 2.0.

- `"hello" + 42` → **error de tipo** en `clx check`. No se convierte
  silenciosamente a `"hello42"`.
- Mismatch de tipos en cualquier operador/binario → error de tipo.
- La conversión es **siempre explícita** vía los intrinsics existentes:
  `int(x)`, `str(x)`, `float(x)`, `bool(x)`, `type(x)`.

## Excepción única y explícita: widening numérico `Int → Float`

Esta es la **única** coerción implícita permitida. Existe en dos formas
verificables:

1. **En asignación**: `var f: Float = 42;` es válido (Int se promueve a Float).
   Implementado en `cls-core/src/middleware/types.rs:84`
   (`(Type::Int, Type::Float) => true`).
2. **En operadores aritméticos mixtos**: `Int + Float` produce `Float`.
   Implementado en `cls-core/src/middleware/typeck/binary.rs:46-50`
   (numeric promotion estándar).

> El widening numérico es predecible (mismo tipo destino siempre), no
> introduce alocaciones ocultas, y no rompe la monomorfización del emisor
> WASM (Float es un tipo primitivo plano, no boxed). Por eso se permite
> como única excepción. **No extender a otros pares de tipos sin discutir
> esta decisión.**

## Lo que NO es coerción implícita

- **Magic methods** (`__add`, `__sub`, `__equals`, `__call`, `__len`, etc.):
  son **overload de operadores para clases de usuario**, no coerción
  primitivo↔objeto. Un `String + Int` no busca `String.__add(Int)`;
  el typeck rechaza el mismatching antes.
- **Asignación a `Any`/`Value`/`JSON`**: estos tipos son universales por
  diseño (cualquier valor CLS cabe en `Value`); no es coerción, es
  widening estructural del sistema de tipos.
- **Literal → tipo base**: `var s = "hola";` infiere `String` (no literal
  type `"hola"`); esto es inferencia de tipo, no coerción de valor.

## Justificación (WASM / JIT)

- Coerción implícita introduce **alocaciones ocultas y dispatch dinámico**
  en hot paths (boxing del primitivo en un wrapper), violando la regla de
  rendimiento de CLS 2.0 (ver `AGENTS.md`, sección "Reglas de trabajo
  derivadas" del JIT).
- Tipos predecibles → el emisor WASM puede **monomorfizar** y emitir acceso
  directo a memoria lineal, sin sobrecarga.
- Comportamiento consistente: todo mismatch es error. No "a veces coerciona,
  a veces no".

## Cambios derivados de esta decisión

- `cls-core/src/middleware/types.rs:84` — el branch `(Type::Int, Type::Float)
  => true` lleva un comentario referenciando este doc, para que un futuro
  dev no lo borre "por limpieza" ni lo extienda a otros pares.
- `docs/lenguaje/tipos.md:26` — enlaza a este doc en lugar de afirmar la
  coerción como una nota suelta.
- `examples/audit/test-features/tests/test-types.clsx` — incluye test de
  regresión del widening (ver al final del archivo).

## Alcance

- **Aplica a**: typeck, emisor WASM/JIT, runtime.
- **No aplica a**: walker deprecado (referencia sintáctica; ver
  `AGENTS.md` §"el JIT es el intérprete objetivo").
- **No se discute**: el widening `Int → Float` ya está implementado y se
  mantiene. Esta decisión **lo blinda** y prohíbe extenderlo.

## Reversibilidad

Baja. Cambiar esta regla requeriría:
- Revertir el branch en `types.rs:84`.
- Cambiar la promoción numérica en `binary.rs:46-50`.
- Actualizar tests.
- Revisar el emisor WASM (el widening permite emitir `f64.convert_i64_s`
  en prologue; sin él, los calls mixtos trapean).

Si en el futuro se quisiera un sistema más permisivo (estilo TypeScript con
coerción de string), debería hacerse como **decisión 002** que reemplace a
esta, no como extensión silenciosa.
