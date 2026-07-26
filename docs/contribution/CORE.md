# cls-core — Compilador

## Estructura

```
cls-core/src/
├── lib.rs              # API pública, re-exports
├── frontend/
│   ├── lexer.rs        # Tokenizador
│   ├── token.rs        # Definiciones de Token, Keyword, Operator, Symbol
│   ├── parser.rs       # Parser recursive descent
│   └── ast.rs          # Nodos del AST (70+ tipos)
├── middleware/
│   ├── types.rs        # Type enum
│   ├── typeck.rs       # Type Checker
│   ├── resolver.rs     # Name Resolver
│   └── optimizer.rs    # Optimizer
├── backend/
│   ├── json.rs         # AST → JSON
│   ├── wasm.rs         # AST → WASM (placeholder)
│   └── visitor.rs      # Trait AstVisitor
├── config/
│   ├── manifest.rs     # ModuleManifest (module.clsconfig)
│   └── types.rs        # Tipos de configuración
└── error/
    ├── diagnostic.rs   # Span, Diagnostic, Severity
    └── mod.rs          # ClsError, ClsResult
```

## Lexer

**Archivo:** `frontend/lexer.rs`

**Struct:** `Lexer`

### Campos:
```rust
pub struct Lexer {
    source: Vec<char>,        // Código fuente como chars
    source_str: String,       // Código fuente original
    pos: usize,               // Posición actual
    line: u32,                // Línea actual
    col: u32,                 // Columna actual
    line_start: usize,        // Inicio de línea actual
    diagnostics: Vec<Diagnostic>, // Errores/warnings
}
```

### Métodos principales:
- `new(source: &str) -> Self` — constructor
- `tokenize() -> ClsResult<Vec<SpannedToken>>` — tokeniza todo
- `next_token() -> ClsResult<SpannedToken>` — siguiente token con span
- `lex_string(delimiter) -> ClsResult<Token>` — parsea strings
- `lex_number() -> ClsResult<Token>` — parsea números
- `lex_identifier_or_keyword() -> ClsResult<Token>` — parsea identificadores
- `lex_operator_or_symbol() -> ClsResult<Token>` — parsea operadores y símbolos

### Flujo de tokenización:
1. `skip_whitespace_and_comments()` — salta espacios y `# comentarios`
2. Detecta el tipo de carácter:
   - `"`, `'`, `` ` `` → string
   - `0-9` → número
   - letra o `_` → identificador/keyword
   - `<` → posible CMX
   - otro → operador o símbolo
3. Envuelve el Token en `SpannedToken { token, span }`

### Tokens generados:
- `Token::IntLiteral(i64)`, `Token::FloatLiteral(f64)`
- `Token::StringLiteral(String)`, `Token::CharLiteral(char)`
- `Token::Identifier(String)`, `Token::Keyword(Keyword)`
- `Token::Operator(Operator)`, `Token::Symbol(Symbol)`
- `Token::Cmx(CmxToken)`, `Token::EOF`

---

## Parser

**Archivo:** `frontend/parser.rs`

**Struct:** `Parser`

### Campos:
```rust
pub struct Parser {
    tokens: Peekable<IntoIter<SpannedToken>>,
    current_token: Token,
    current_span: Span,
    diagnostics: Vec<Diagnostic>,
}
```

### Método principal:
- `parse() -> ClsResult<Module>` — parsea todo el archivo

### Gramática (recursive descent):

```
Module       → Statement*
Statement    → VarDecl | ConstDecl | FunctionDecl | If | While | Loop
             | For | ForEach | Switch | Try | With | Return | Break
             | Continue | ClassDecl | StructureDecl | InterfaceDecl
             | ModuleDecl | NamespaceDecl | Import | FromImport
             | Include | Config | Expression

Expression   → Assignment
Assignment   → Conditional (('=' | '+=' | '-=' | '*=' | '/=') Assignment)?
Conditional  → LogicalOr
LogicalOr    → LogicalAnd ('|' LogicalAnd)*
LogicalAnd   → Equality ('&' Equality)*
Equality     → Comparison (('==' | '!=') Comparison)*
Comparison   → Term (('<' | '>' | '<=' | '>=') Term)*
Term         → Factor (('+' | '-') Factor)*
Factor       → Unary (('*' | '/' | '%' | '**') Unary)*
Unary        → ('-' | '!' | '~') Unary | Call
Call         → Primary ( '(' args ')' | '.' member | '::' member | '[' index ']' )*
Primary      → Literal | Identifier | '(' expr ')' | '[' ... ']' | '{' ... '}' 
             | 'if' '(' cond ')' 'then' '(' a ')' 'else' '(' b ')' | 'true' | 'false' | 'me'
```

---

## AST (Abstract Syntax Tree)

**Archivo:** `frontend/ast.rs`

### Nodos principales:

**Module:**
```rust
pub struct Module {
    pub statements: Vec<Statement>,
    pub span: Span,
}
```

**Statement (enum con 30+ variantes):**
- `VarDecl(VarDecl)` — `var x = 5`
- `ConstDecl(VarDecl)` — `const PI = 3.14`
- `FunctionDecl(FunctionDecl)` — `function f() -> Int { ... }`
- `If(IfStatement)` — `if (cond) { ... } elif { ... } else { ... }`
- `While(WhileStatement)` — `while (cond) { ... }`
- `Loop(Block)` — `loop { ... }`
- `For(ForStatement)` — `for (init; cond; update) { ... }`
- `ForEach(ForEachStatement)` — `for each x in (arr) { ... }`
- `Switch(SwitchStatement)` — `switch (val) { case ... }`
- `Try(TryStatement)` — `try { ... } catch { ... }`
- `With(WithStatement)` — `with x in (val) { ... }`
- `Return(Option<Expression>)` — `return expr`
- `Break`, `Continue`
- `ClassDecl(ClassDecl)` — `class Name { ... }`
- `StructureDecl(StructureDecl)` — `structure Name { ... }`
- `InterfaceDecl(InterfaceDecl)` — `interface Name { ... }`
- `ModuleDecl(ModuleDecl)` — `module Name { ... }`
- `NamespaceDecl(NamespaceDecl)` — `namespace Name { ... }`
- `Import(ImportStatement)` — `import "path" as alias`
- `FromImport(FromImportStatement)` — `from "path" import a, b`
- `Include(IncludeStatement)` — `include "path"`
- `Expression(Expression)` — expresión como statement
- `Config(ConfigDirective)` — `#config(...)`
- `Cmx(CmxElement)` — elementos JSX

**Expression (enum con 18 variantes):**
- `Literal(Literal)` — números, strings, booleanos
- `Identifier(String, Span)` — nombres de variables
- `Binary(BinaryExpr)` — `a + b`, `a == b`
- `Unary(UnaryExpr)` — `-x`, `!x`
- `Call(CallExpr)` — `f(args)`
- `MemberAccess(MemberAccessExpr)` — `obj.member`
- `Index(IndexExpr)` — `arr[i]`
- `Array(ArrayExpr)` — `[1, 2, 3]`
- `Record(RecordExpr)` — `{key: value}`
- `ArrowFunction(ArrowFunctionExpr)` — `(x) -> x * 2`
- `Conditional(ConditionalExpr)` — `if (c) then (a) else (b)`
- `Assignment(AssignmentExpr)` — `x = 5`, `x += 1`
- `Parenthesized(Box<Expression>, Span)` — `(expr)`
- `StringInterpolation(StringInterpolation)` — `"$name"`
- `Cmx(CmxElement)` — `<Tag>...</Tag>`
- `NamespaceAccess(String, String, Span)` — `Module::member`

---

## Type Checker

**Archivo:** `middleware/typeck.rs`

**Struct:** `TypeChecker`

### Campos:
```rust
pub struct TypeChecker {
    config: TypesConfig,                // Configuración de tipos
    diagnostics: Vec<Diagnostic>,       // Errores/warnings
    scopes: Vec<HashMap<String, Type>>, // Tabla de símbolos
    current_return_type: Option<Type>,  // Tipo de retorno esperado
}
```

### Métodos:
- `check(module) -> ClsResult<()>` — verifica todo el módulo
- `resolve_type_annotation(ann) -> Type` — resuelve TypeKind → Type
- Camina el AST y verifica:
  - Variables declaradas tengan tipo compatible con el valor
  - Parámetros de funciones coincidan con la llamada
  - Operadores reciban tipos compatibles
  - Returns coincidan con el tipo declarado

---

## Name Resolver

**Archivo:** `middleware/resolver.rs`

**Struct:** `NameResolver`

### Campos:
```rust
pub struct NameResolver {
    scopes: Vec<Scope>,
    diagnostics: Vec<Diagnostic>,
}
```

### Métodos:
- `resolve(module) -> ClsResult<()>` — resuelve todo el módulo
- Camina el AST y registra cada símbolo en su scope
- Verifica que las variables usadas estén definidas

---

## Optimizer

**Archivo:** `middleware/optimizer.rs`

**Struct:** `Optimizer`

### Métodos:
- `optimize(module)` — optimiza el AST in-place
- Realiza:
  - Constant folding (evalúa expresiones constantes)
  - Dead code elimination (elimina código inalcanzable)
