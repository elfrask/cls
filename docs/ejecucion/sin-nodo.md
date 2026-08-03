# Ejecutar un script sin nodo (embedding)

Puedes ejecutar CLS desde tu propio programa Rust usando el **core** y el
**runtime** directamente, sin el CLI `clx` ni el runtime `clxr`. Esto es útil
para incorporar CLS como lenguaje de scripting en otra aplicación.

## Dependencias

Añade a tu `Cargo.toml`:

```toml
[dependencies]
cls-core = { path = "cls-core" }
cls-runtime = { path = "cls-runtime" }
```

## Programa mínimo

```rust
use cls_core::frontend::{Lexer, Parser};
use cls_runtime::{Intrinsics, Interpreter, ModuleResolver};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        function main(args: String[]) -> int {
            print("Hola desde CLS embebido");
            return 0;
        };
    "#;

    // 1. Tokenizar y parsear
    let tokens = Lexer::new(source).tokenize()?;
    let module = Parser::new(tokens).parse()?;

    // 2. Crear el intérprete
    //    - Intrinsics::empty() sin globales extra, o desktop_defaults(args) para print/input/args
    //    - ModuleResolver con stdlib core (math, json, async)
    let mut interp = Interpreter::new(
        Intrinsics::empty(),
        ModuleResolver::new().with_core_stdlib(),
    );

    // 3. Ejecutar y llamar a main
    interp.execute(&module)?;
    let code = interp.call_main()?;
    std::process::exit(code);
}
```

## Intrinsics

`Intrinsics` aporta los globales del programa:

- `Intrinsics::empty()` — sin globales (no habrá `print`, `input`, `args`).
- `Intrinsics::desktop_defaults(args: Vec<String>)` — `print`, `input`, `args`.
- `Intrinsics::add(nombre, valor)` — agrega un global personalizado.

```
let mut intr = Intrinsics::desktop_defaults(vec!["--flag".to_string()]);
intr.add("miGlobal", Value::Int(42));
```

## ModuleResolver

El `ModuleResolver` decide cómo se consiguen los módulos. El runtime es
agnóstico:

- `ModuleResolver::new()` — sin internals.
- `.with_core_stdlib()` — agrega `math`, `json`, `async`.
- `.add_internal(nombre, valor)` — agrega un módulo interno.
- `.set_external(closure)` — hook para resolver módulos de usuario
  (por ejemplo, leyendo archivos). El closure recibe el path y el entorno, y
  devuelve `Ok(Some(Value))`, `Ok(None)` (no encontrado) o `Err`.

Ejemplo de hook que lee `.clsx`:

```
let resolver = ModuleResolver::new().with_core_stdlib()
    .set_external(|path: String, _env| -> ClsResult<Option<Value>> {
        match std::fs::read_to_string(format!("{}.clsx", path)) {
            Ok(source) => {
                let mut interp = Interpreter::new(Intrinsics::empty(),
                    ModuleResolver::new().with_core_stdlib());
                Ok(Some(interp.load_module_source(&path, &source)?))
            }
            Err(_) => Ok(None),
        }
    });
```

## Reporte de errores

Para mostrar errores con el trazo completo, usa el formateo centralizado:

```
use cls_runtime::{ErrorFormat, format_error};

match interp.execute(&module) {
    Ok(_) => {}
    Err(e) => {
        let report = interp.build_error_report(e);
        eprintln!("{}", format_error(&report, &ErrorFormat::Console));
    }
}
```

Elige el formato con `ErrorFormat::Plain` / `Console` / `Html` / `Json`.

## Verificación de tipos

Si quieres verificar tipos antes de ejecutar:

```
use cls_core::middleware::TypeChecker;
use cls_core::config::types::TypesConfig;

let mut checker = TypeChecker::new(TypesConfig { check: true, strict: true, ..Default::default() });
checker.check(&module)?;
let diagnostics = checker.diagnostics();
```

## Notas

- El `Interpreter` no es `Send`; ejecuta en un solo hilo (el scheduler de
  corrutinas de `clxr`).
- Los módulos del nodo (`fs`, `http`, `Lib`) NO forman parte del runtime; si los
  necesitas, inyéctalos con `add_internal` desde tu código.
