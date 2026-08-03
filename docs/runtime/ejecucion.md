# Ejecución: el intérprete

El runtime de CLS (`cls-runtime`) ejecuta el AST directamente con un
**tree-walker**: recorre las sentencias y expresiones del árbol y las evalúa una
por una, sin compilar a código máquina.

## Pipeline

```
fuente (String)
  → Lexer   → Vec<Token>
  → Parser  → Module (AST)
  → Interpreter::execute(&Module) → Value
```

## El `Interpreter`

El `Interpreter` mantiene el estado de la ejecución:

- **`env`** — el entorno (scopes de variables).
- **`call_stack`** — la pila de llamadas (para el trazo de errores).
- **`import_trace`** — los imports en curso (para el trazo de errores).
- **`flow`** — la señal de flujo (`Normal`, `Return`, `Break`, `Continue`).
- **`self_stack`** — frames `me`/`super` durante llamadas a métodos.
- **`classes`**, **`structs`**, **`method_tables`** — tipos definidos y métodos
  de primitivos.

## Ciclo de vida

1. `Interpreter::new(Intrinsics, ModuleResolver)` — crea el intérprete, define
   los intrinsics globales (`print`, `input`, `type`, `len`, `toString`, etc.)
   y los módulos internos.
2. `interpreter.execute(&module)` — ejecuta las sentencias de nivel superior.
3. `interpreter.call_main()` — busca `main`, la llama con los argumentos y
   devuelve el código de salida.

## Ejecución de sentencias

Cada sentencia tiene un manejador:

- Declaraciones: `execute_var_decl`, `execute_function_decl`,
  `execute_class_decl`, `execute_enum_decl`, `execute_structure_decl`,
  `execute_module_decl`, ...
- Control de flujo: `execute_if`, `execute_while`, `execute_loop`,
  `execute_for`, `execute_for_each`, `execute_switch`, `execute_try`, ...
- Imports: `execute_import`, `execute_from_import`.

`execute_block` detiene la iteración si la señal de flujo no es `Normal` (por
ejemplo, después de un `return`).

## Evaluación de expresiones

Cada expresión tiene un manejador:

- Literales e identificadores.
- `evaluate_binary` — operadores (incluye magic methods `__add`, `__equals`,
  etc., y el operador `is`/`in`).
- `evaluate_call` — llamadas a funciones, métodos y objetos `__call`.
- `evaluate_member_access` — acceso a campos/métodos, enums, records, clases y
  métodos de tipos primitivos.
- `evaluate_index` — indexado de arrays, tuplas, records y objetos `__get`.
- `evaluate_cmx` — construcción de `CmxValue`.

## Funciones nativas

Las funciones nativas de Rust se representan como `FunValue` con
`FunKind::Native`, que encierra una closure `Fn(&[Value]) -> ClsResult<Value>`.
Las funciones de usuario son `FunKind::User` (AST) con un entorno léxico
capturado opcional (closures).

## Cargas de módulos

`Interpreter::load_module_source(nombre, source)` compila un módulo, lo ejecuta
en un scope aislado, recolecta SOLO los símbolos `export` y devuelve un record.
Es el punto central de carga de módulos del runtime.

## Configuración del runtime

La config (modo, límites, sandbox) se pasa con `interpreter.set_config(...)`.
El modo actual de ejecución es `pure-ast` (tree-walker). El modo `jit` está
planeado (ver `docs/future/`).
