# Testing

## Ejecutar los tests

```
cargo test --workspace
```

Ejecuta los tests unitarios de todos los crates. La salida muestra el número de
tests por crate:

```
test result: ok. 61 passed ...   # cls-core
test result: ok. 77 passed ...   # cls-runtime
```

## Dónde viven los tests

Los tests unitarios están en módulos `#[cfg(test)] mod tests { ... }` dentro de
cada archivo fuente. Cobertura principal:

- `cls-core/src/frontend/lexer.rs` — tokenización.
- `cls-core/src/frontend/parser.rs` — parseo.
- `cls-core/src/middleware/types.rs` — reglas de tipos (assignability).
- `cls-core/src/middleware/typeck.rs` — el verificador (fuente → diagnostics).
- `cls-core/src/middleware/resolver.rs` — resolución de nombres.
- `cls-core/src/error/mod.rs` — errores y spans.
- `cls-runtime/src/environment.rs` — scopes.
- `cls-runtime/src/value.rs` — valores.
- `cls-runtime/src/interpreter.rs` — ejecución (source → Value).
- `cls-runtime/src/stdlib/math.rs`, `json.rs`, `primitive.rs` — módulos y
  métodos de primitivos.
- `cls-runtime/src/vfs/` — sistema de archivos virtual.
- `cls-runtime/src/error_report.rs` — formatos de error.

## Estilo de tests

### Verificador de tipos (`typeck.rs`)

Helper que parsea y verifica un source, devolviendo los diagnostics:

```
fn check_source(src: &str, strict: bool) -> Vec<Diagnostic> {
    let toks = Lexer::new(src).tokenize().expect("tokenize");
    let module = Parser::new(toks).parse().expect("parse");
    let config = TypesConfig { check: true, strict, ..Default::default() };
    let mut tc = TypeChecker::new(config);
    tc.check(&module).expect("check no debe fallar");
    tc.diagnostics().to_vec()
}
```

Luego se verifica el número de errores:

```
#[test]
fn tuple_invalid_slot() {
    let d = check_source("function f() { var a: (Int, String) = (1, 2); };", true);
    assert_eq!(count_errors(&d), 1);
}
```

### Intérprete (`interpreter.rs`)

Helper que parsea y ejecuta un source, devolviendo el valor de la última
sentencia:

```
fn run(src: &str) -> ClsResult<Value> {
    let toks = Lexer::new(src).tokenize().expect("tokenize");
    let module = Parser::new(toks).parse().expect("parse");
    let mut interp = Interpreter::new(Intrinsics::empty(), ModuleResolver::new().with_core_stdlib());
    interp.execute(&module)
}
```

El source de prueba termina en una expresión cuyo valor se verifica:

```
#[test]
fn tuple_index() {
    let v = run_ok("(10, 20, 30)[1]");
    assert_eq!(v, Value::Int(20));
}
```

Para errores esperados:

```
#[test]
fn tuple_immutable() {
    assert!(run("var t = (1, 2); t[0] = 9;").is_err());
}
```

### Módulos y métodos primitivos

Extrae la función nativa de la tabla o módulo y la invoca con un slice de
`Value`:

```
fn method_for(t: PrimitiveType, name: &str) -> MethodFn { ... }
let f = method_for(PrimitiveType::String, "upper");
assert_eq!(call(&f, vec![Value::String("hola".into())]), Value::String("HOLA".into()));
```

## Ejemplos de regresión

Además de los tests unitarios, `examples/tests/` contiene scripts `.clsx` que se
ejecutan como verificación manual:

```
clx run examples/tests/test-methods.clsx
clx run examples/tests/test-types.clsx
clx check --strict examples/tests/test-types.clsx
```

Los tests de importación se ejecutan desde `examples/tests/` (los imports son
relativos al directorio de trabajo):

```
clx run test-imports.clsx
```

## Convenciones

- Cada test debe ser autónomo y no depender del orden de ejecución.
- Usa asserts con mensajes descriptivos: `assert_eq!(x, y, "contexto: {:?}", d)`.
- Para parsear fuentes en los tests, usa `Lexer` + `Parser` directamente (no el
  CLI).
- Si agregas una feature, agrega al menos un test positivo y uno negativo.
