# Arquitectura de cls-core

`cls-core` es el lenguaje en sí: frontend, middleware, configuración y errores.
Es completamente agnóstico al entorno.

## Frontend

### token

Define las categorías léxicas:

- `Keyword` — palabras reservadas.
- `Operator` — operadores (`+`, `==`, `is`, `in`, ...).
- `Symbol` — puntuación (`(`, `{`, `;`, ...).
- `CmxToken` — tokens del marcado CMX (OpenTag, CloseTag, attrs, texto).

`Token` es una de estas categorías o un literal (entero, flotante, cadena,
booleano, carácter, identificador).

### lexer

Convierte el texto en tokens. Maneja:

- Cadenas (doble, simple, backtick) e interpolación.
- Números.
- Identificadores y keywords.
- Comentarios (`#`).
- Operadores y símbolos (incluidos los multi-carácter: `->`, `==`, `**`, `+=`).
- **CMX**: el lexer reconoce `<Tag ...>` como marcado, con desambiguación para
  los genéricos de tipo (`<T>`, `<T=Int>`) mediante lookahead y *backtracking*
  (si un `<` no es un tag CMX válido, se emite como operador `<`).
- Spans (línea/columna) en cada token.

### parser

Construye el AST. Características:

- **Precedencia** por niveles: assignment → conditional → or → and → bitwise →
  equality (`==`, `!=`, `is`, `in`) → comparison → shift → additive →
  multiplicative → unary → call/member/index → primary.
- Declaraciones: `var`/`const`/`let`, `function` (con genéricos `<T>`), `class`
  (con herencia `:`/`extends`/`(Base)` y type params), `structure`, `interface`
  (con type params y campos), `enum`, `alias`, `module`, `namespace`, imports.
- Expresiones: literales, arrays, **tuplas** `(a, b)`, records `{k: v}`,
  funciones flecha (con lookahead), CMX, interpolación, ternarios, asignaciones
  compuestas, postfix `++`/`--`.
- **Anotaciones de tipo** completas: tuplas `(Int, String)`, uniones `"a" | "b"`,
  tipos función `(Int) -> Int`, genéricos `Foo<T>`, extracción `T["campo"]`/`T[0]`,
  `Record<K, V>`, phantom `!T`.
- Comentarios `# @...` para documentación (usados por los type maps).

### ast

Define el árbol de sintaxis: `Module`, `Statement`, `Expression`, `VarDecl`,
`FunctionDecl`, `ClassDecl`, `EnumDecl`, `TypeAliasDecl`, `InterfaceDecl`,
`TypeAnnotation`, etc. Todo es serializable (`serde`) para el dump y el
empaquetado futuro.

## Middleware

### types

El sistema de tipos compile-time:

```
Type::Int | Float | String | Bool | Char | Any | ...
Type::Array(Box<Type>)
Type::Tuple(Vec<Type>)        // heterogéneo por posición
Type::Record(Box<Type>, Box<Type>)
Type::Fun(Vec<Type>, Box<Type>)
Type::Union(Vec<Type>)
Type::Literal(LitVal)          // "red", 5, true
Type::Named(String, Vec<Type>)
Type::Infer(usize)
```

`is_assignable_to` define las reglas de asignación (ver `lenguaje/tipos.md`).

### typeck

El verificador de tipos. Recorre el AST y produce `Diagnostic`s (errores y
advertencias con span). Mantiene:

- **Scopes** de tipos (variables en el ámbito actual).
- `interfaces` — las interfaces declaradas (campos/métodos como AST sin resolver).
- `enums` — los nombres de enums (para que `Color.Rojo` tenga tipo `Color`).

Resuelve anotaciones con `resolve_type_annotation` (y `resolve_annotation_with`
para sustituir genéricos con bindings). `check_with_prelude` registra los tipos
de módulos importados antes de verificar el principal.

### resolver

El resolvedor de nombres: verifica que las variables estén definidas y gestiona
scopes. Produce errores de "variable no definida" y conoce símbolos de función,
clase, enum, etc.

### optimizer

Pase de optimización del AST (preliminar).

## Configuración

- `config/manifest.rs` — `ModuleManifest` (cls.json).
- `config/types.rs` — `TypesConfig`, `CompilerConfig`, `InterpreterConfig`,
  `FeaturesConfig`, `WarningsConfig`, `RuntimeMemoryConfig`, `SandboxConfig`,
  `GcConfig`.

## Errores y colores

- `error/` — `ClsError`, `Span`, `Diagnostic`, `StackFrame`, `extract_line_col`.
- `ansi/` — códigos ANSI centralizados (`fg`, `bold`).

## Backend

- `backend/json.rs` — dump del AST a JSON.
- `backend/wasm.rs` — backend WASM (planeado).
- `backend/visitor.rs` — recorrido del AST.
