# Cómo agregar un módulo interno

Un módulo interno es una biblioteca accesible vía `import "nombre"` sin
descargar nada: vive en el resolver del nodo. Hay dos clases:

- **Core** - agnóstico al entorno, siempre disponible
  (`cls-runtime/src/stdlib/`). Hoy: `math`, `json`, `async`.
- **Nodo desktop** - dependen del entorno (`nodos/clx/src/modules/`).
  Hoy: `fs`, `http`, `Lib`, `os`, `path`, `process`, `time`, `random`.

Un módulo es en runtime un `Value::Record` de funciones nativas
(`FunValue::new_native`) y constantes.

## Módulo core

1. **Crear `cls-runtime/src/stdlib/mi_modulo.rs`** con una función
   `pub fn module() -> Value` que construya el `Record`:

```rust
use crate::value::{FunValue, Value};
use cls_core::error::{ClsError, ClsResult};
use std::collections::HashMap;

pub fn module() -> Value {
    let mut m = HashMap::new();
    m.insert("PI".into(), Value::Float(3.14159));
    m.insert("doble".into(), Value::Fun(FunValue::new_native("doble", vec!["x".into()], |a| {
        let v = a.first().ok_or(ClsError::RuntimeError("doble: esperaba 1 arg".into()))?;
        match v {
            Value::Int(i) => Ok(Value::Int(i * 2)),
            _ => Err(ClsError::RuntimeError("doble: esperaba int".into())),
        }
    })));
    Value::Record(m)
}
```

Ejemplo real: `cls-runtime/src/stdlib/math.rs::module()`
(+ `json.rs`, `async_.rs`).

2. **Declararlo en `cls-runtime/src/stdlib/mod.rs`**:

```rust
pub mod mi_modulo;
```

3. **Registrarlo en `ModuleResolver::with_core_stdlib`**
   (`cls-runtime/src/resolver.rs`):

```rust
pub fn with_core_stdlib(mut self) -> Self {
    self.internals.insert("math".into(), crate::stdlib::math::module());
    self.internals.insert("json".into(), crate::stdlib::json::module());
    self.internals.insert("async".into(), crate::stdlib::async_::module());
    self.internals.insert("mi_modulo".into(), crate::stdlib::mi_modulo::module());
    self
}
```

4. **Crear el `.clsi`** en `cls-runtime/clsi/mi_modulo.clsi` - declaración
   de tipos que alimenta el LSP, `clx maptype` y el typeck del JIT
   (ver `cls-runtime/clsi/math.clsi` como plantilla):

```
# @title mi_modulo
# @description Descripcion del modulo.

# @description Valor de prueba
var PI: float;

# @description Duplica un entero
function doble(x: int) -> int {};
```

## Módulo del nodo desktop

1. **Crear `nodos/clx/src/modules/mi_modulo.rs`** - igual que core, pero
   puede recibir dependencias del nodo (VFS, args) por parámetro, como
   `fs::module(vfs)` o `process::module(app_args)`.
2. **Declararlo en `nodos/clx/src/modules/mod.rs`** (`pub mod mi_modulo;`).
3. **Registrarlo en `make_desktop_resolver`**
   (`nodos/clx/src/subcommands/run.rs`) con
   `resolver.add_internal("mi_modulo", crate::modules::mi_modulo::module());`
   - esto lo expone al **walker** (`clx run --ast-walker`).
4. **JIT** - el walker no ejecuta el JIT: el JIT necesita sus propias host
   functions:
   - En `cls-jit/src/host.rs` (cuerpos genéricos) y en
     `cls-jit/src/wasmtime_rt.rs` (`register_host_functions` registra cada
     `env.*` en el `Linker`), o
   - vía el canal `env.host_call(id, ptr, n)` con `HostCallHandler` del nodo.
   - El typeck (`cls-core/src/middleware/typeck.rs`) valida los accesos:
     `check_member_access` resuelve `mi_modulo.func()` contra las tablas de
     miembros por módulo y `module_arity` valida la aridad de cada función.
5. **`.clsi`** en `cls-runtime/clsi/mi_modulo.clsi` - además de LSP/maptype,
   es la fuente de las firmas que usa el typeck del JIT (ver
   `fs.clsi`, `os.clsi`, `time.clsi`, etc.).

## Notas

- `ModuleResolver::resolve` (cache -> internals -> external hook -> error):
  un `add_internal` basta para que `import "mi_modulo"` funcione en el
  walker; el JIT resuelve los imports con su propio mapa de hosts.
- Los `.clsi` viven en `cls-runtime/clsi/` (uno por módulo + `types.clsi`);
  `clx maptype` los procesa igual que `.clsx`.
- Los errores de aridad/argumentos usan `ClsError::RuntimeError` con
  mensaje `"<fn>: esperaba N arg(s)"` (patrón de `stdlib/math.rs`).