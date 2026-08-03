# Cómo agregar un módulo interno

Los módulos internos son valores disponibles para `import "nombre"`. Se dividen
en **core** (siempre disponibles, agnósticos) y **del nodo** (dependen del
entorno).

## Módulo core (siempre disponible)

Los módulos core viven en `cls-runtime/src/stdlib/` (`math.rs`, `json.rs`,
`async_.rs`). Son agnósticos: no tocan el sistema.

### 1. Crea el archivo

`cls-runtime/src/stdlib/mi_modulo.rs`:

```rust
use std::collections::HashMap;
use crate::value::{FunValue, Value};
use cls_core::error::ClsResult;

fn funcion(args: &[Value]) -> ClsResult<Value> {
    // args[0] es el primer argumento
    Ok(Value::Int(42))
}

pub fn module() -> Value {
    let mut entries = HashMap::new();
    entries.insert("miFuncion".to_string(), Value::Fun(FunValue::new_native(
        "miFuncion", vec!["x".to_string()], funcion,
    )));
    Value::Record(entries)
}
```

### 2. Decláralo en `stdlib/mod.rs`

```
pub mod mi_modulo;
```

### 3. Regístralo en el resolver

`cls-runtime/src/resolver.rs`, en `with_core_stdlib`:

```
self.internals.insert("mi_modulo".into(), crate::stdlib::mi_modulo::module());
```

### 4. Documenta

- `docs/runtime/biblioteca-estandar.md` — la tabla del módulo.
- `cls-runtime/clsi/mi_modulo.clsi` — la interfaz de tipos (para type maps):
  ```
  # @title mi_modulo
  # @description Descripción.
  function miFuncion(x: int) -> int {};
  ```

Luego regenera los type maps: `clx maptype cls-runtime/clsi -o ...`.

## Módulo del nodo (entorno)

Los módulos del nodo (como `fs`, `http`, `Lib`) viven en
`nodos/clx/src/modules/`. Interactúan con el sistema operativo. El core/runtime
no los conocen.

### 1. Crea el archivo

`nodos/clx/src/modules/mi_modulo.rs` con la misma estructura (`module() -> Value`).

### 2. Decláralo

En `nodos/clx/src/modules/mod.rs` (o `main.rs` si no hay `mod.rs`):

```
pub mod mi_modulo;
```

### 3. Inyéctalo en el resolver del nodo

En `nodos/clx/src/subcommands/run.rs`, `make_desktop_resolver`:

```
resolver.add_internal("mi_modulo", crate::modules::mi_modulo::module());
```

### 4. Documenta y agrega el `.clsi`

Igual que los módulos core, en `docs/runtime/biblioteca-estandar.md` y
`nodos/clx/clsi/...` o `cls-runtime/clsi/` según corresponda.

## Notas

- Las funciones nativas del módulo son `Fn(&[Value]) -> ClsResult<Value>`.
- Si una función necesita estado (por ejemplo, acceso al sistema de archivos),
  la closure puede capturar esa dependencia: `función` recibe `args` pero la
  closure que la envuelve captura el `Arc<VfsResolver>`, etc.
- Regenera los type maps cuando agregues funciones nuevas.
