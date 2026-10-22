# Reporte — `match`: expresión de matching, enums con payload, ternarios y `<-`

> Feature de lenguaje implementada por fases (ver `agent-context/MATCH_PLAN.md`).
> Ciclo **dev → correcciones → Juez** (gate) en cada fase; Juez final de la feature.
> **Veredicto final del Juez: APROBADO** (tras un primer **RECHAZO** por B1/B2 que
> se corrigió).

- **Commits:** `9b14850` (F1), `29ead52` (F2), `5059ff5` (F3+F4), `bca2db7` (F5),
  `737c802` (F6), `7b1965c` (fixes B1/B2 del Juez).
- **Regla de autocontención (§2):** todo en `cls-core` (parser/typeck/emisor WASM),
  sin `host_calls` nuevos, sin punteros expuestos, sin boxing, sin trucos.

---

## Qué se implementó

| Fase | Contenido |
|---|---|
| **F1** | `match` como expresión/sentencia. 4 modos: `is` (tipo), `in` (valor), `if` (condición), sin-keyword (**patrón**). Binding `<-` del scrutinee. Narrowing por arma. `default` opcional; **match vacío = error**; tipo = primera arma. Emisor con `BlockType::Result`. |
| **F2** | **Enums con payload** (`enum R { ok(String), err(int,String), s1 }`). Construcción `R::v(...)`; patrón declarativo `ok(x)`/`err(a,b)`/`s1, s2`. Layout heap `[def_id][idx][arity][payload…]` (unit-only sigue empacado i64). **Destructuring-let estático** `var R::ok(v) = c;`. |
| **F3** | **`?:` estilo JS** (`cond ? a : b`, asociativo a derecha) y **`if`-expresión** (`if (c) { * <- a } else { * <- b }`, `else` obligatorio). |
| **F4** | **`arrowReturn` `<-`** léxico: `*`/`**` (ternario: rama de `if`-expr o cuerpo de arma de `match`) y `match`/`match*`/`match**` (match por conteo). Atraviesa loops; **sin target = error de compilación**; **sin cross-function**. |
| **F5** | **Unión temporal**: acceso a miembro sobre `Type::Union` de clases/structs/enums, con **dispatch por `class_id`/`def_id`** y offset por miembro. Index sobre unión de arrays. |
| **F6** | **Protocolo de match para clases**: `__match() -> E` (namespace de patrones = variantes de `E`) + `__unpack(tag) -> T \| (…)` (payload computado). `is`→identidad de clase, `in`→`__equals`, `if`→`__bool`. |

Documentación: `docs/lenguaje/control-de-flujo.md`, `sintaxis.md`, `enums.md`,
`tipos.md`, `magics.md`, `oop.md`; VS Code grammar/snippets.

## Bug del lenguaje encontrado (en F1) y corregido

Durante F1 se detectó y corrigió un **bug preexistente** del lexer: el `switch`
sobre `String` comparaba el **handle** en vez del contenido (ya resuelto antes para
`==`); se unificó con `__intr_str_eq` por contenido (reutilizado por `match`).

## Bugs del **Juez final** (RECHAZO → corregidos)

| ID | Fase | Síntoma | Causa raíz | Fix |
|----|------|---------|------------|-----|
| **B1** | F3/F4 | `if`-expr **anidada** en posición de sentencia dentro de un bloque-target → **WASM inválido** (`expected i64 but nothing on stack`) | El validador WASM no propaga la divergencia a través del `end` de un `if`/`match` interno: tras su `end` la caída vuelve a considerarse alcanzable y el bloque `(result)` queda sin valor | Emitir `Instruction::Unreachable` al cerrar la rama/arma cuando `dead_flow` (materializa la divergencia; pila polimórfica). `emitter/expressions.rs`, `emitter/matching.rs` |
| **B2** | F5 | `match (v) is` sobre **unión de builtins**: falso positivo silencioso (`int\|String` → primera arma) y **WASM inválido** con ancho mixto (`int\|float`) | `match_type_static` respondía "la unión contiene el target" → toda arma verdadera; no hay tag/boxing para discriminar | Rechazo estático inmediato; después reemplazado por **uniones boxeadas `[tag][valor]`** (ver `emitter/tagged_union.rs`), que permiten `match is`, `is`, `print`, `str`/`toString` por tag real. `int`/`float` sobre la unión siguen rechazados. |
| **B2-doc** | — | `tipos.md` afirmaba que `str(int \| float)` funciona | era **falso** | Doc corregida + límite documentado |

Tests agregados por los fixes: `ifexpr-nested`, `ifexpr-nodeliver`,
`match-union-builtin`, `str-union`, `conv-union` + negativos en lang-features.

## Verificación (al cierre)

- `examples/test/jit-test/run-tests.ps1`: **78 PASS, 0 FAIL, 0 SKIP, 0 NEW**.
- `run-availible.ps1`: **24 PASS, 0 FAIL, 1 SKIP**.
- `cargo test --workspace`: **0 fallos**.
- Diferencial/regresión: `conformance` **2302 + fuzz 20000**, `brainfuck` **OK**,
  `clsh` **22/25** (3 de arquitectura), `sqlite-orm` **OK**, `lang-features` **OK**
  (14/14 negativos), `dns` **OK**.
- Juez: verificó con **casos propios** los 4 modos, payload, ternarios/`<-`,
  unión de clases, protocolo de clases; anti-falso-verde OK; `git diff` de
  `cls-jit`/`cls-internals` vacío; `switch` intacto; 24 magics intactos; enums
  unit intactos; sin `host_calls`/punteros/boxing. **APROBADO tras B1/B2**.

## Gaps documentados (errores limpios, no crashes)

- `for each` sobre enum **con payload** → error explícito.
- Constante de enum en patrones de **modo valor** (`in`) → solo literales.
- `else if` dentro de `if`-expresión no encadenable.
- Construcción de enum importado `mod::Enum::v(...)` no parseable (el matching de
  enum importado sí funciona).
- **Destructuring-let de clase** (runtime) no implementado (stretch); `match`
  cubre la extracción. Sintaxis propuesta documentada.

## Nota de infraestructura (pre-existente)

Los `.expect` están en git con **LF+BOM**; con `core.autocrlf=true` un `git
checkout` normal los reescribe a CRLF y el runner (que no normaliza) da **falso
rojo**. Restaurar con `git -c core.autocrlf=false checkout -- <expect>`.
