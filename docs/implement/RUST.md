# Implementación desde Rust

CLS puede ser integrado como librería en cualquier proyecto Rust.

## Agregar dependencia

```toml
[dependencies]
cls-core = { path = "ruta/a/cls/cls-core" }
cls-runtime = { path = "ruta/a/cls/cls-runtime" }
```

## Pipeline completo

```rust
use cls_core::frontend::{Lexer, Parser};
use cls_runtime::{Intrinsics, ModuleResolver, Interpreter};

fn main() {
    let source = r#"
        function main(args: String[]) -> int {
            print("Hello from embedded CLS!");
            return 0;
        }
    "#;

    // 1. Tokenizar
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    // 2. Parsear
    let mut parser = Parser::new(tokens);
    let module = parser.parse().unwrap();

    // 3. Configurar runtime
    let intrinsics = Intrinsics::desktop_defaults(vec![]);
    let resolver = ModuleResolver::new()
        .with_core_stdlib(); // math + json

    // 4. Ejecutar
    let mut interpreter = Interpreter::new(intrinsics, resolver);
    interpreter.execute(&module).unwrap();
    interpreter.call_main().unwrap();
}
```

## Solo type checking

```rust
use cls_core::middleware::TypeChecker;
use cls_core::config::types::TypesConfig;

let config = TypesConfig {
    check: true,
    strict: false,
    ..Default::default()
};
let mut checker = TypeChecker::new(config);
checker.check(&module).unwrap();

for diag in checker.diagnostics() {
    eprintln!("[{}] {}", diag.severity, diag.message);
}
```

## Solo AST

```rust
use cls_core::backend::JsonBackend;

let json = JsonBackend::new().emit(&module).unwrap();
println!("{}", json);
```
