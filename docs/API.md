# CLS 2.0 — API de los crates

---

## `cls-core` — Compilador

```rust
use cls_core::frontend::{Lexer, Parser};
use cls_core::middleware::{TypeChecker, NameResolver, Optimizer};
use cls_core::config::manifest::ModuleManifest;
use cls_core::config::types::TypesConfig;
use cls_core::backend::JsonBackend;

// Tokenizar
let mut lexer = Lexer::new(source_code);
let tokens = lexer.tokenize()?;

// Parsear
let mut parser = Parser::new(tokens);
let mut module = parser.parse()?;

// Type check (configurable)
let config = TypesConfig::default();
let mut checker = TypeChecker::new(config);
checker.check(&module)?;

// Resolver nombres
let mut resolver = NameResolver::new();
resolver.resolve(&module)?;

// Optimizar
let optimizer = Optimizer::new();
optimizer.optimize(&mut module);

// Exportar AST a JSON
let json = JsonBackend::new().emit(&module)?;
```

---

## `cls-runtime` — Motor de ejecución

```rust
use cls_runtime::{Interpreter, Value, Environment, Sandbox};

// Ejecutar (tree-walker)
let args = vec!["--port".to_string(), "8080".to_string()];
let mut interpreter = Interpreter::new(args);
let result = interpreter.execute(&module)?;

println!("Resultado: {}", result);

// Sandbox habilitado
let mut sandbox = Sandbox::new();
sandbox.allow_fs = false;
sandbox.allow_net = true;
sandbox.check_net_access()?;
```

---

## Flujo típico en Rust

```rust
use cls_core::frontend::{Lexer, Parser};
use cls_core::middleware::{TypeChecker, NameResolver};
use cls_core::config::manifest::ModuleManifest;
use cls_runtime::Interpreter;

// 1. Cargar configuración
let manifest = ModuleManifest::from_file("module.clsconfig")?;

// 2. Leer código fuente
let source = std::fs::read_to_string("src/main.ccls")?;

// 3. Frontend: Lex + Parse
let tokens = Lexer::new(&source).tokenize()?;
let mut module = Parser::new(tokens).parse()?;

// 4. Middleware: Type check + Resolve
let mut checker = TypeChecker::new(manifest.compiler.types.clone());
checker.check(&module)?;

let mut resolver = NameResolver::new();
resolver.resolve(&module)?;

// 5. Backend: Ejecutar
let mut interpreter = Interpreter::new(args);
let exit_code = interpreter.execute(&module)?;
```
