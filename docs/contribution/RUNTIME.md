# cls-runtime — Motor de Ejecución

## Estructura

```
cls-runtime/src/
├── lib.rs              # API pública, re-exports
├── value.rs            # Value enum y FunValue
├── environment.rs      # Environment (scopes anidados)
├── interpreter.rs      # Tree-walker interpreter
├── intrinsics.rs       # Intrinsics (top-level configurable)
├── resolver.rs         # ModuleResolver
├── gc.rs               # Garbage Collector (placeholder)
├── sandbox.rs          # Sandbox (restricciones)
├── modules.rs          # ModuleManager (.clsapp loader)
├── host_api.rs         # Host API (funciones del host)
├── ffi.rs              # FFI exports (C ABI)
├── error.rs            # Re-export de ClsError
└── stdlib/
    ├── math.rs         # Módulo math (core)
    └── json.rs         # Módulo json (core)
```

---

## Value

**Archivo:** `value.rs`

### Value enum
Representa todos los valores runtime de CLS.

```rust
pub enum Value {
    Int(i64),                           // Números enteros
    Float(f64),                         // Números decimales
    String(String),                     // Cadenas
    Bool(bool),                         // Booleanos
    Char(char),                         // Caracteres
    Null,                               // Nulo
    Void,                               // Sin valor (retornos void)
    Array(Vec<Value>),                  // Arrays
    Record(HashMap<String, Value>),     // Records/Objetos
    Fun(FunValue),                      // Funciones
    Unknown,                            // Tipo desconocido
    Cmx(Box<CmxValue>),                 // Elementos JSX
}
```

### Métodos:
- `type_name() -> &str` — nombre del tipo
- `is_truthy() -> bool` — ¿es truthy?
- `to_string() -> String` — representación string

### FunValue
Representa una función (nativa o de usuario).

```rust
pub struct FunValue {
    pub name: String,
    pub kind: FunKind,
}

pub enum FunKind {
    Native {
        params: Vec<String>,
        func: Arc<dyn Fn(&[Value]) -> ClsResult<Value>>,
    },
    User {
        params: Vec<Parameter>,   // Parámetros del AST
        body: Block,              // Cuerpo de la función
    },
}
```

### CmxValue
```rust
pub struct CmxValue {
    pub tag: String,
    pub props: HashMap<String, Value>,
    pub children: Vec<Value>,
}
```

---

## Environment

**Archivo:** `environment.rs`

Gestiona los scopes anidados (tabla de símbolos por ámbito).

```rust
pub struct Environment {
    scopes: Vec<Scope>,  // Stack de scopes (global al fondo)
}

struct Scope {
    variables: HashMap<String, Value>,
}
```

### Métodos:
- `new() -> Self` — scope global vacío
- `push_scope()` / `pop_scope()` — entrar/salir de scope
- `define(name, value)` — definir variable en scope actual
- `get(name) -> Option<&Value>` — buscar desde el scope más reciente
- `set(name, value)` — modificar variable existente
- `contains(name) -> bool` — ¿existe en algún scope?
- `all() -> HashMap<String, Value>` — todas las variables del scope global

### Orden de búsqueda:
```
get("x") → último scope (bloque)
        → scope de función
        → scope global
        → None
```

---

## Interpreter

**Archivo:** `interpreter.rs`

Tree-walker que ejecuta el AST directamente.

```rust
pub struct Interpreter {
    env: Environment,
    resolver: ModuleResolver,
    diagnostics: Vec<Diagnostic>,
    args: Vec<String>,
}
```

### Constructor:
```rust
Interpreter::new(intrinsics: Intrinsics, resolver: ModuleResolver) -> Self
```

1. Registra los globales del nodo (print, input, ...)
2. Registra intrínsecos del core (int, str, float, ...)
3. Almacena args del CLI

### Métodos principales:

**`execute(module) -> ClsResult<Value>`**
Recorre todos los statements del módulo. Para cada uno:
- `VarDecl` → evalúa expresión, define en env
- `FunctionDecl` → crea FunValue, registra en env
- Expressions → evalúa recursivamente

**`call_main() -> ClsResult<i32>`**
1. Busca `main` en el entorno
2. Construye `args` como `Value::Array`
3. Llama `call_function_value(main, [args])`
4. Retorna el código de salida

**`evaluate_expression(expr) -> ClsResult<Value>`**
Evalúa recursivamente cualquier expresión:
- Literales → valor directo
- Identificadores → lookup en environment
- Binarias → evalúa izquierda y derecha, aplica operador
- Llamadas → evalúa callee y args, ejecuta función
- MemberAccess → lookup en record
- Index → acceso a array/record

**`call_function_value(callee, args) -> ClsResult<Value>`**
- `FunKind::Native` → llama el closure Rust
- `FunKind::User` → push scope, define params, ejecuta body, pop scope

**`execute_block(block) -> ClsResult<Value>`**
Push scope, ejecuta statements, pop scope.

---

## Intrinsics

**Archivo:** `intrinsics.rs`

Funciones y valores top-level configurables por el nodo.

```rust
pub struct Intrinsics {
    pub globals: HashMap<String, Value>,
    pub args: Vec<String>,
}
```

### Métodos:
- `desktop_defaults(args) -> Self` — print + input con stdout/stdin
- `empty() -> Self` — sin nada (el nodo agrega todo)
- `add(name, value) -> &mut Self` — agregar un global

---

## ModuleResolver

**Archivo:** `resolver.rs`

Sistema de resolución de módulos con caché.

```rust
pub struct ModuleResolver {
    internals: HashMap<String, Value>,           // Módulos built-in
    external: Option<Box<dyn Fn(String, ...)>>,  // Hook externo
    cache: HashMap<String, Value>,               // Módulos ya cargados
}
```

### Métodos:
- `with_core_stdlib() -> Self` — agrega math + json
- `add_internal(name, module)` — agrega/quita módulo
- `set_external(closure)` — hook para módulos de usuario
- `resolve(path, env) -> ClsResult<Value>` — busca módulo

### Orden de resolución:
```
1. Cache → ¿ya se importó?
2. Internals → ¿está en el Map del nodo?
3. External → llamar al hook del nodo
4. Error → "módulo no encontrado"
```

---

## Flujo de ejecución de una función

```
call_main()
    ├─ env.get("main") → Value::Fun
    └─ call_function_value(fun, [args_array])
        │
        ├─ FunKind::Native
        │   └─ func(&[args_array])   ← closure Rust
        │
        └─ FunKind::User
            ├─ env.push_scope()      ← scope de función
            ├─ for param in params:
            │   └─ env.define(param.name, args[i])
            ├─ execute_block(body)
            │   ├─ env.push_scope()  ← scope de bloque
            │   ├─ for stmt in body:
            │   │   └─ execute_statement(stmt)
            │   └─ env.pop_scope()
            └─ env.pop_scope()       ← scope de función
```
