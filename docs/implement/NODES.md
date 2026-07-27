# Crear tu propio Nodo

Un nodo es un ejecutable o librería que ejecuta CLS en un entorno específico.

## Componentes a configurar

1. **Intrinsics**: funciones y valores top-level
2. **ModuleResolver**: qué módulos están disponibles
3. **External hook**: cómo cargar módulos de usuario

## Ejemplo: Nodo Desktop (ccls)

```rust
use cls_runtime::{Intrinsics, ModuleResolver, Interpreter, Environment, Value};

fn make_desktop_resolver() -> ModuleResolver {
    let mut resolver = ModuleResolver::new()
        .with_core_stdlib(); // math + json

    // Módulos propios del nodo
    resolver.add_internal("fs", fs_module());
    resolver.add_internal("http", http_module());

    // Hook externo: buscar archivos .clsx
    resolver.set_external(|path, env| {
        let candidate = format!("{}.clsx", path);
        if let Ok(source) = std::fs::read_to_string(&candidate) {
            compile_and_load(&source)
        } else {
            None
        }
    });

    resolver
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let app_args = args[1..].to_vec();

    let intrinsics = Intrinsics::desktop_defaults(app_args);
    let resolver = make_desktop_resolver();

    let source = std::fs::read_to_string("app.clsx").unwrap();
    let tokens = cls_core::frontend::Lexer::new(&source).tokenize().unwrap();
    let module = cls_core::frontend::Parser::new(tokens).parse().unwrap();

    let mut interpreter = Interpreter::new(intrinsics, resolver);
    interpreter.execute(&module).unwrap();
    interpreter.call_main().unwrap();
}
```

## Ejemplo: Nodo Web (sin FS)

```rust
let mut resolver = ModuleResolver::new()
    .with_core_stdlib();
// Sin fs, sin http

// Hook remoto
resolver.set_external(|path, _env| {
    fetch_module_from_cdn(path)
});
```

## Ejemplo: Personalizando Intrinsics

```rust
let mut intrinsics = Intrinsics::desktop_defaults(vec![]);

// Agregar funciones personalizadas
intrinsics.add("getVersion", Value::Fun(version_fn));
intrinsics.add("APP_NAME", Value::String("MyApp".into()));
intrinsics.add("logError", Value::Fun(error_logger));

// En CLS:
// print(getVersion());
// print(APP_NAME);
```

## Ejemplo: Crear nodo en otro lenguaje

Cuando cls-runtime esté compilado a WASM:

```python
# Python
import wasmtime

runtime = wasmtime.Module.from_file("cls-runtime.wasm")
instance = wasmtime.Instance(runtime)

source = 'function main(args) -> int { print("Hello!"); return 0; }'
result = instance.exports.compile_and_run(source, [])
```
