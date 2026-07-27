# Pipeline de Compilación

El pipeline de CLS procesa código fuente a través de las siguientes etapas:

```
Código fuente (.clsx)
    │
    ▼
┌─────────────────────────────────────────┐
│ 1. LEXER (cls-core/src/frontend/lexer.rs)│
│    ────────────────────────────────────  │
│    Carácter por carácter → Tokens       │
│    ────────────────────────────────────  │
│    Entrada: String (código fuente)       │
│    Salida:  Vec<SpannedToken>           │
│    SpannedToken { token, span }          │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 2. PARSER (cls-core/src/frontend/parser.rs)│
│    ────────────────────────────────────  │
│    Tokens → AST (Abstract Syntax Tree)  │
│    ────────────────────────────────────  │
│    Recursive descent parser             │
│    Entrada: Vec<SpannedToken>           │
│    Salida:  Module { statements }       │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 3. TYPE CHECKER (middleware/typeck.rs)  │
│    ────────────────────────────────────  │
│    AST → Verificación de tipos          │
│    ────────────────────────────────────  │
│    Recorre el AST verificando tipos     │
│    Produce diagnósticos (errors/warns)  │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 4. NAME RESOLVER (middleware/resolver.rs)│
│    ────────────────────────────────────  │
│    AST → Resolución de nombres y scopes │
│    ────────────────────────────────────  │
│    Verifica que variables y funciones   │
│    estén definidas antes de su uso      │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 5. OPTIMIZER (middleware/optimizer.rs)  │
│    ────────────────────────────────────  │
│    AST → AST optimizado                 │
│    ────────────────────────────────────  │
│    Constant folding, dead code removal  │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ 6. BACKEND                              │
│    ────────────────────────────────────  │
│    AST → Ejecución / Compilación        │
│    ────────────────────────────────────  │
│    ┌─ Tree-walker (interpreter.rs)      │
│    ├─ JSON dump (json.rs)               │
│    └─ WASM codegen (wasm.rs, futuro)    │
└─────────────────────────────────────────┘
```

## Flujo de ejecución (tree-walker)

```
Module { statements }
    │
    ▼
Interpreter::execute(module)
    │  Recorre cada statement
    │
    ├─ VarDecl       → evalúa expresión, define variable
    ├─ FunctionDecl  → crea FunValue, registra en el entorno
    ├─ If/While/For  → evalúa condición, ejecuta bloque
    ├─ Return        → evalúa expresión, retorna valor (sin cortar bloque)
    ├─ Expression    → evalúa expresión recursivamente
    ├─ Import        → resuelve módulo, define alias
    └─ ...
    │
    ▼
Interpreter::call_main()
    │
    ├─ Busca "main" en el entorno
    ├─ Construye args como Value::Array
    └─ call_function_value(main, args)
        │
        ├─ FunKind::Native  → llama closure de Rust
        └─ FunKind::User    → push scope, define params,
                              execute_block(body), pop scope
```
