# Reporte — CLS estático (eliminación de `Any`) + retrofit con features nuevas

> Lote de cierre: **quitar `Any` como anotación** (para que CLS sea totalmente
> estático) y **usar las features nuevas** (`match`, enums con payload, ternarios
> `?:`/`if`-expr, `arrowReturn <-`, unión, protocolo `__match`/`__unpack`) en las
> pruebas existentes. Ciclo dev → correcciones → Juez.

- **Commits:** `5c6a194` (elimina `Any` + fallout), `d018081` (retrofit),
  `34d4794` (1er fix de literales — **revertido**), `c8b5087` (**fix corregido**).
- **Veredicto del Juez:** **APROBADO** (tras un primer **RECHAZO**, ver abajo).

---

## 1. `Any` eliminado como anotación

- **Core**: `cls-core/src/middleware/typeck/types.rs::resolve_annotation_with`:
  `TypeKind::Any` → **error de compilación** claro
  (`` `Any` ya no es un tipo válido: CLS es estático. Usá un tipo concreto … o
  `Value`/`JSON`/`Cmx` ``). El `Type::Any` **interno** (fallback de inferencia,
  `print(val: …)`) se conserva.
- **Fallout**: `Record<String, any>` → `Record<String, Value>`; `json.parse` →
  `JSON`; `Any[]` → `Value[]`. En `*.clsx`/`*.clsi` (LSP), docs, tabla de tipos,
  grammar VS Code y snippets.
- **Fixes de emisión necesarios** (los tapaba `Any`): `toString()` sobre
  `Value`/`Json` (antes solo `Any`/`Unknown`), y `length`/`size` de un `JSON`
  (tag 7) devuelve el tamaño estructural.
- **Observación (GAP 19, preexistente)**: un **local** anotado `Value`
  (`var v: Value = …`) no despacha por tag: `print`/`str` dan basura o WASM
  inválido. El camino soportado (leer de `Record`/`JSON`) sí funciona.
  Registrado en `findings/19-value-local.clsx` + `gaps.txt`.

## 2. Retrofit con features nuevas (resultados idénticos)

| Prueba | Features usadas | Resultado |
|---|---|---|
| brainfuck | `enum Op` + `match` (patrón/valor), `?:` | OK byte a byte |
| conformance | `match` (dispatch), `?:`, `if`-expr + `<-` | 2302/2302 + fuzz 20000/20000 |
| clsh | `match` valor/patrón sobre `enum Tok`, `?:` | 22/25 (mismas 3 de arquitectura) |
| sqlite-orm | `match` (`cellOf`/`bind`), `?:` | dump 553090 B / CHECK 1582 B |
| dns | `match`, `match is` (unión), `<-`, `?:` | 360/8/173/10 |

Workarounds quitados: `OP_*` (brainfuck), `K_*` (clsh), `int(TYPE_A)` (dns),
ramas muertas. `switch` como **feature** intacto (varios *usos* de dispatch se
migraron a `match`, según §5.6).

## 3. Bug del retrofit (F-18) y su corrección

- **F-18**: `match` con armas que devuelven `const` de distinto valor
  (`const A=1; const B=2; match(x) in {1->A, 2->B, default->0}`) **no compilaba**:
  la 1ª arma fijaba `Literal(1)` y `Literal(2)` no le era asignable.
- **1er intento** (`34d4794`): ensanchar incondicionalmente con `literal_base()` →
  **rechazado por el Juez**: rompía la asignación a **uniones de literales**
  (`alias Flags = 1|2; var y: Flags = c ? A : B;`).
- **Fix final** (`c8b5087`): `Type::join_compatible` = asignable en cualquier
  dirección **o mismo tipo base**, **sin ensanchar** (preserva el literal). Usado
  en `check_match` (armas + `default`), `check_ternary`/`check_conditional` y
  `check_if_expr`. Cubre `Literal(1)` vs `Literal(2)` **y** `Literal(1)` vs `Int`
  (literal sintáctico), sin romper uniones.

## 4. Verificación (al cierre)

- `examples/test/jit-test/run-tests.ps1`: **78 PASS / 0 FAIL** ·
  `run-availible.ps1`: **24 PASS / 1 SKIP** · `cargo test --workspace`: **0 fallos**.
- Runners: brainfuck **OK** · conformance **2302 + fuzz 20000** · clsh **22/25** ·
  sqlite-orm **OK** · dns **OK** · lang-features **OK** (1 resuelto, **3 GAP**).
- Juez: verificó con casos propios el fix de literales, la eliminación de `Any`,
  el retrofit y las reglas (`cls-internals` sin diff, sin `host_calls`, `switch`
  intacto, oráculos sin cambios salvo el rename `18-dict-any`→`18-json`);
  anti-falso-verde OK. **APROBADO**.

## 5. Gaps / observaciones

- **19 — local `Value`**: preexistente (ver §1).
- **Asimetría preexistente** en `if`-expr/ternario: literal sintáctico infiere
  `Int` y `const` infiere `Literal` (afecta el tipado de la expresión, no la
  asignación). No es regresión del lote.
- **`Any` interno** sigue como fallback de inferencia (p. ej. ternario de tipos no
  relacionados devuelve `Type::Any`), asignable a todo. Preexistente.
