# Cómo agregar una feature al lenguaje

Este documento describe el recorrido típico para agregar una feature a CLS,
usando como ejemplo un nuevo operador binario. Ajusta los pasos según el tipo
de feature.

## 1. Decide el alcance

- ¿Afecta solo el **verificador** (compile-time), como alias/uniones/phantom?
- ¿Afecta el **runtime** (valores, operadores, sentencias), como enums/tuplas?
- ¿Afecta el **parseo** (nueva sintaxis), el **lexer** (nuevo token) o ambos?

Esto determina qué archivos tocar.

## 2. Lexer y tokens

Si introduces una palabra reservada u operador nuevo:

- `cls-core/src/frontend/token.rs` — agrega el `Keyword`/`Operator` y su
  representación en `Display`.
- `cls-core/src/frontend/lexer.rs` — agrega el mapeo en `lex_identifier_or_keyword`
  (para palabras) o en el manejo de símbolos (para operadores).

## 3. AST

- `cls-core/src/frontend/ast.rs` — define el nodo del AST (struct o variant en
  `Statement`/`Expression`/`TypeKind`). Todo debe derivar
  `Debug, Clone, Serialize, Deserialize`. Si es un `Statement`, agrégalo al
  `Display`.

## 4. Parser

- `cls-core/src/frontend/parser.rs` — agrega el dispatch en `parse_statement`
  (para sentencias) o el manejo en la función de precedencia correspondiente
  (para operadores), y la función `parse_...`.
- Actualiza los constructores de nodos que cambien.

## 5. Middleware (si es compile-time)

- `cls-core/src/middleware/types.rs` — agrega el `Type` y las reglas de
  `is_assignable_to`.
- `cls-core/src/middleware/typeck.rs` — verificación, inferencia, registro de
  tipos y resolución de anotaciones.
- `cls-core/src/middleware/resolver.rs` — registro de símbolos nuevos.

## 6. Runtime (si afecta ejecución)

- `cls-runtime/src/value.rs` — agrega el `Value` (si es un valor nuevo) y su
  `type_name`, `is_truthy`, `to_string`, `PartialEq`.
- `cls-runtime/src/interpreter.rs` — agrega el manejador de sentencia
  (`execute_...`) o de expresión (`evaluate_...`), y los accesos necesarios
  (member access, index, etc.).
- `cls-runtime/src/environment.rs` — si necesita nuevos comportamientos de scope.

## 7. Nodos y herramientas

- Si la feature es un módulo interno o un intrinsics, actualiza el nodo
  (`nodos/clx/src/...`) o `cls-runtime/src/intrinsics.rs`.
- Si introduce sintaxis, actualiza el grammar de la extensión
  (`.vscode/extensions/ccls-lang/syntaxes/clsx.tmLanguage.json`), los snippets y
  los type maps (`.clsi` + `clx maptype`).
- Si introduce un tipo, agrega su interfaz en `cls-runtime/clsi/types.clsi`.

## 8. Tests

- **Tests unitarios**: agrega `#[test]` en el módulo de tests del archivo que
  cambió (ver `desarrollo/testing.md`).
- **Ejemplo**: crea o actualiza un archivo en `examples/tests/`.

## 9. Documentación

- Agrega la feature a `docs/` (por ejemplo, `lenguaje/tipos.md`,
  `lenguaje/oop.md`, ...).
- Actualiza `agent-context/` si corresponde (planes de features).

## 10. Verificación

```
cargo build --workspace
cargo test --workspace
```

Asegúrate de que no haya warnings nuevos.

## Recorrido rápido (operador `in`)

La feature del operador `in` (contención) se implementó así:

1. `token.rs`: `Operator::In` + display.
2. `parser.rs`: detección de `Keyword::In` en `parse_equality` → `BinaryExpr`.
3. `interpreter.rs`: manejo de `Operator::In` en `evaluate_binary_values`
   (nativo para arrays/tuplas/records/strings, `__contains` para objetos).
4. Test unitario en `interpreter.rs` y ejemplo.
