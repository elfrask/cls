# El tree-walker (DEPRECADO)

> **DEPRECADO.** El tree-walker se eliminará tras CLS 2.0-dev1. `clx run`
> usa el **JIT** por defecto; el walker queda como **referencia sintáctica**
> del lenguaje. No inviertas tiempo en paridad con él: lo que importa es que
> el JIT soporte la feature.

## Activar

```ps
clx run --ast-walker main.clsx
```

Imprime una advertencia en stderr:

```
[DEPRECADO] El intérprete AST-walker está deprecado y se desaconseja su uso.
Se eliminará tras CLS 2.0-dev1. Usa `clx run` (JIT) para ejecutar programas.
```

## Implementación

Vive en `cls-runtime/src/interpreter.rs` (junto a `environment.rs` y
`value.rs`). Ejecuta el AST directamente, sin compilación intermedia.

### `Interpreter`

```rust
pub struct Interpreter {
    env: Environment,            // scopes anidados
    resolver: ModuleResolver,    // módulos (cache → internals → hook)
    diagnostics: Vec<Diagnostic>,
    args: Vec<String>,           // args de la aplicación
    exports: HashSet<String>,    // símbolos exportados
    source_file: String,
    import_trace: Vec<ImportFrame>,
    call_stack: Vec<StackFrame>, // se conserva en error (no se popea)
    flow: Flow,                  // Normal | Return(Value) | Break | Continue
    config: Option<ModuleManifest>,
    structs: HashMap<String, StructDef>,
    classes: HashMap<String, ClassDef>,
    self_stack: Vec<SelfFrame>,  // { obj, current_class } para me/super
    method_tables: HashMap<PrimitiveType, Table>, // métodos de primitivos (sin boxing)
    native_backends: HashMap<String, Arc<dyn NativeBackend>>, // extension
    target: Target,              // directiva `when`
    when_declared: HashSet<String>,
}
```

### `Value`

```rust
pub enum Value {
    Int(i64), Float(f64), String(String), Bool(bool), Char(char),
    Null, Void,
    Array(Vec<Value>), Tuple(Vec<Value>), Record(HashMap<String, Value>),
    Fun(FunValue), Struct(Box<StructInstance>), Promise(Promise),
    Class(Box<ClassDef>), Object(Box<ClassInstance>),
    EnumDef(Box<EnumDef>), Enum(Box<EnumValue>),
    Unknown,
    Cmx(Box<CmxValue>),
}
```

### `Flow`

```rust
enum Flow { Normal, Return(Value), Break, Continue }
```

- `execute_block()` detiene la iteración si `flow != Normal`.
- `execute_loop/while/for` manejan `Break`/`Continue`/`Return`.
- `call_function_value` captura `Return` y limpia el flow con `mem::replace`.

### Errores

`build_error_report` → `show_runtime_error` (`cls-runtime/src/error_report.rs`)
imprime el trace completo: import_trace + call stack numerado con código
fuente por frame + el frame del error con caret (ver `runtime/errores.md`).

## Features que el walker soporta y el JIT aún no

- **async/await** — `Promise` con `delay`, `all` y `race`; poll de
  `Pollable` (`cls-runtime/src/stdlib/async_.rs`). El JIT no compila `await`.
- **Magic methods completos** (24) — `__iter`, `__next`, `__neg`, `__not`,
  `__repr`, `__toString`, `__len`, `__int`, `__float`, `__bool`, `__compare`,
  `__equals`, `__add`, `__sub`, `__mul`, `__div`, `__mod`, `__pow`,
  `__contains`, `__get`, `__set`, `__call`, `__toJson`, `__type`.
- **Extensión vía backends nativos dinámicos** — `NativeBackend`
  (`cls-runtime/src/ffi.rs`) registrado por el nodo
  (`set_native_backend` / `register_native_backend`); el JIT compila
  `extension` solo con el backend nativo del nodo (`DynamicBackend`).

## Módulos del walker

`ModuleResolver` (`cls-runtime/src/resolver.rs`):
cache → internals → external hook → error.

- Core (`with_core_stdlib`): `math`, `json`, `async`.
- Nodo desktop (`clx run --ast-walker`, `nodos/clx/src/subcommands/run.rs`):
  agrega `fs`, `http`, `Lib`, `os`, `path`, `process`, `time`, `random` y un
  hook externo que lee el source del módulo y lo carga con
  `load_module_source`.

`load_module_source` (`cls-runtime/src/interpreter.rs:1689`): ejecuta el
módulo en un scope aislado (guarda/restaura `exports`, `env` y `resolver`) y
devuelve **solo los símbolos marcados `export`** como `Value::Record`.