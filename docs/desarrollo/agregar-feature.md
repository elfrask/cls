# Cómo agregar una feature

Una feature nueva atraviesa las capas del compilador, del análisis de tipos
y, si aplica al JIT, del emisor WASM. Estructura:

```
cls-core/src/frontend/   lexer.rs, token.rs, parser.rs, ast/ (un archivo por tipo)
cls-core/src/middleware/ typeck/ (check_*), types.rs, resolver.rs, optimizer.rs
cls-core/src/backend/    wasm/ (engine/ + emitter/), json.rs, visitor.rs
cls-runtime/src/walker/  interpreter.rs (evaluate_*), value.rs
```

## Pasos por capa

### 1. Tokens (`token.rs` / `lexer.rs`)

Solo si la feature introduce sintaxis nueva:

- Agrega la variante de token y su `Display` legible (los errores muestran
  el símbolo real, no el Debug).
- En `lexer.rs`, produce el token desde la fuente.

### 2. AST (`ast/`)

Define el nodo (p. ej. `Expression::Foo(FooExpr)` o
`Statement::Foo(...)`) con su `Span` en `frontend/ast/` (re-exportado en
`ast/mod.rs`). Los spans son obligatorios: alimentan el type map y los caret
de error.

### 3. Parser (`parser.rs`)

- Crea `fn parse_foo(&mut self) -> Result<FooExpr>`.
- Los errores de sintaxis usan `self.syntax_err(msg)` (fábrica
  centralizada `ClsError::syntax_at`), mensaje limpio + span estructurado.
- Si hay ambigüedad con lookahead (arrows, tuplas vs paréntesis), el
  parser ya tiene patrones: `is_arrow_function`, check directo en LParen,
  depth tracking.

### 4. Typeck (`middleware/typeck/`)

- `check_foo(...)` produce el `Type` de la expresión y registra
  `types_by_span` (el type map `Span -> Type`).
- Si la feature es un miembro de módulo nuevo, actualiza las tablas de
  miembros por módulo en `check_member_access` y `module_arity`.
- En modo estricto las asignaciones incompatibles son ERROR con span real.

### 5. Emisor WASM (`backend/wasm/`) - el JIT

Lo que permite `clx run`:

- Implementa `emit_foo(...)`; el subset homogéneo: `Int` -> `i64`,
  `Float` -> `f64`, `Bool` -> `i32`, `String` -> `i64` `(ptr << 32) | len`,
  referencias -> `i64` (bump allocator).
- Si la operación tiene equivalente en internals precompiladas
  (`cls-internals`, funciones `__intr_*`), emite `call __intr_*` por nombre
  (con host fallback si no está fusionada).
- Si el emisor no la soporta, produce un error explícito
  `El JIT (subconjunto WASM) aún no soporta ...` con `compile_at(msg, span)`.
- Lo que el walker soporta pero no el emisor no frena la feature: **el JIT
  manda** (ver Directiva en `AGENTS.md`).

### 6. Walker (`cls-runtime/src/walker/interpreter.rs`) - opcional

Solo si quieres la referencia sintáctica en el tree-walker:
`evaluate_foo(...)` + `Value` en `value.rs`. El walker está deprecado; no
inviertas tiempo en paridad.

### 7. Tests

- Unit tests en cada capa tocada (lexer, parser, typeck, emisor).
- QA: `examples/audit/features/NN-nombre.clsx` más
  `examples/audit/test-features/jit-test/availible/` si entra en el subset.
- **Si la feature es solo JIT**, va en `examples/jit-examples/` o
  `jit-test/availible/` (no en la carpeta de paridad walker).

## Reglas

1. Usa fábricas de error centralizadas (`syntax_at` / `compile_at`) y
   spans estructurados; nunca incrustes `(línea N, columna M)` en el mensaje.
2. El typeck es la fuente de verdad del emisor (`types_by_span`); el
   backend no re-deriva tipos.
3. Rendimiento: sin boxing ni dispatch dinámico en el runtime del JIT; el
   compilador debe poder monomorfizar.
4. Documenta en `docs/` solo lo implementado; si cambia comportamiento,
   actualiza `docs/` en el mismo cambio.
5. Verifica con `clx check --strict` y `clx run` (JIT por defecto) antes de
   commitear (`feat(jit): ...`, español, ver `desarrollo/contribuir.md`).

## Referencia de ejemplo

- CMX: tokens en `lexer.rs` (`lex_cmx`), parseo en
  `parser.rs::parse_cmx`, evaluación en `walker/interpreter.rs::evaluate_cmx`,
  tipo en el typeck, emisión en `backend/wasm/` (host functions `cmx_*`).
- Errores del emisor no soportado: `compile_at` en `backend/wasm/`
  (p. ej. `await`, índices dinámicos en records con shape).
- Internals WASM: agregar una función `__intr_*` = implementar en
  `cls-internals/wasm/src/<area>.rs` + firmar en `abi.rs` (ver
  `agregar-modulo-interno.md`).