# Resolvers de módulos

El runtime resuelve módulos a través del `ModuleResolver` (`cls-runtime/src/resolver.rs`).
El core y el runtime son **agnósticos**: no saben de dónde salen los módulos ni
qué internos existen. Es el **nodo** (o tu aplicación embebida) quien configura
el resolver.

## El contrato

`ModuleResolver::resolve(path, env)` sigue este orden:

1. **Caché** — si el módulo ya se cargó, devuelve la copia cacheada.
2. **Internals** — busca en los módulos internos registrados por nombre.
3. **Hook externo** — delega en el closure configurado con `set_external`.
4. **Error** — "módulo no encontrado".

## API

```
let mut resolver = ModuleResolver::new();        // vacío
resolver.add_internal("mi_modulo", value);       // módulo interno
resolver.set_external(|path, env| -> ClsResult<Option<Value>> { ... });
```

- `with_core_stdlib()` — agrega `math`, `json`, `async`.
- `add_internal(nombre, valor)` — agrega un módulo interno por nombre.
- `set_external(closure)` — el hook que decide cómo conseguir módulos de
  usuario.

## Cómo desarrollar un resolver de módulos

Para que tu aplicación resuelva `import "mod"`:

### 1. Define el hook externo

El hook recibe el path del import (sin extensión) y devuelve un valor de módulo
(usualmente un `Value::Record` con los símbolos exportados):

```
use cls_runtime::{Intrinsics, Interpreter, ModuleResolver, Value};
use cls_core::error::ClsResult;

fn hook(path: String, _env: &mut cls_runtime::Environment) -> ClsResult<Option<Value>> {
    // Ejemplo: leer <path>.clsx del disco y ejecutarlo con el runtime centralizado
    match std::fs::read_to_string(format!("{}.clsx", path)) {
        Ok(source) => {
            let mut interp = Interpreter::new(
                Intrinsics::empty(),
                ModuleResolver::new().with_core_stdlib(),
            );
            Ok(Some(interp.load_module_source(&path, &source)?))
        }
        Err(_) => Ok(None),
    }
}
```

### 2. Configura el resolver

```
let resolver = ModuleResolver::new().with_core_stdlib().set_external(hook);
let mut interp = Interpreter::new(Intrinsics::desktop_defaults(vec![]), resolver);
```

### 3. El usuario importa

```
import "mod" as m;
```

## Reglas de diseño

- **El nodo decide cómo conseguir los módulos.** El runtime solo ejecuta.
- **La carga de un módulo** (compilar + ejecutar aislado + recolectar exports)
  es responsabilidad del runtime (`Interpreter::load_module_source`). El hook
  del nodo solo obtiene el *source* (o el valor ya construido).
- **Los internals del nodo** (`fs`, `http`, `Lib`) se inyectan con
  `add_internal`; el core/runtime no los conocen.
- Para **empaquetar** (`.clsapp`/`.clslib`), el resolver se usa para descubrir
  todos los módulos que se convertirán en `.clbin` dentro del paquete.

## ClsLibResolver (librerías `.clslib`)

Las librerías compiladas usan un resolver separado, `ClsLibResolver`
(`cls-runtime/src/clslib.rs`), configurable por nodo:

- Busca en: directorio de trabajo, `$CLS_LIB_PATH` y las rutas del nodo.
- Devuelve los bytes del `.clslib` para su indexación por hash (SHA-256) y carga.
- Equivale a un `.dll`/`.so` para CLS.

El runtime lo usa para `Lib.load(ruta)`.
