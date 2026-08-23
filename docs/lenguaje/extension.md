# Extensiones nativas (`extension` - FFI)

`extension` declara símbolos de librerías del sistema (C ABI) para llamarlos
desde CLS. Las funciones se declaran **sin cuerpo** (terminan en `;`).

## Declaración

```clx
extension "libm" as C {
    function sqrt(x: CDouble) -> CDouble;
    function pow(x: CDouble, y: CDouble) -> CDouble;
    export function strlen(s: CString) -> CInt;   # exportable
};
```

- La librería se escribe como string; los nombres conocidos se mapean al
  archivo real del SO (ver más abajo). También vale una ruta directa:

```clx
extension "msvcrt.dll" as C {
    function strlen(s: CString) -> CInt;
    function atoi(s: CString) -> CInt;
};
```

- `export function` (y `export structure`/`export var`) marca el símbolo como
  exportable a otros módulos.
- Dentro del bloque solo se permiten `function`, `structure` y `var`.

Ejemplos: `examples/audit/test-features/extension-demo.clsx`,
`examples/audit/test-features/native-lib.clsx`.

## Tipos C

| Tipo | ABI |
|---|---|
| `CString` | `char*` (String CLS -> buffer null-terminated) |
| `CInt` / `CUInt` | `i32` / `u32` |
| `CShort` / `CUShort` | `i16` / `u16` |
| `CLong` / `CULong` | `i64` / `u64` (long de 64 bits) |
| `CChar` / `CUChar` | `i8` / `u8` |
| `CDouble` / `CFloat` | `f64` / `f32` |
| `CPtr` | puntero (`void*`) |
| `CRecord` | record CLS como ptr al layout `[cap][len][(key,val,tag)*24]` |
| `CArray` | array CLS como ptr al layout `[cap][len][elems*8]` |
| `CStruct` | struct CLS como ptr al layout contiguo (offsets) |
| `Struct(nombre)` | layout C de una `structure` declarada |
| `Bool` | `i32` (0/1) |
| `Void` | sin retorno |
| `Any` | sin anotación: el backend decide |

Nota: **`CFloat` no está soportado** por el dispatcher (`f32`); usar
`CDouble`.

## Valores estructurados (`CRecord` / `CArray` / `CStruct`)

Estos tipos viajan como **puntero al layout canónico de la memoria lineal**
(el de `HostCtx`, igual que usan los hosts JIT y los bindings):

```
string  : (ptr<<32)|len
array   : [cap:i64][len:i64][elems*es]     (es=4 bool/char, 8 resto)
record  : [cap:i64][len:i64][(key,val,tag)*24]
struct  : ptr + offsets contiguos
```

Tags runtime de los valores dentro de records: `0=int 1=string 2=float 3=bool
4=char 6=array 7=record`.

- **Arg**: el valor CLS (Record/Array/Struct) se escribe en el layout y el DLL
  recibe su puntero. El wrapper del JIT traduce el offset de la memoria lineal
  del módulo a la dirección host (la memoria lineal es una alocación del host),
  así el DLL lee/escribe el mismo layout en su espacio de direcciones.
- **Retorno**: el DLL escribe el layout (idealmente in-place sobre la memoria
  del módulo) y CLS lo vuelve a usar como record/array. El puntero que devuelve
  el DLL debe caer dentro de la memoria lineal del módulo (contrato zero-copy);
  si devuelve un buffer propio, CLS lo recibe como `Int` crudo (no re-usable).
- **Records con shape** (`var r = {a:1,b:"x"}` inferido) se emiten como struct
  contiguo, no como hashmap: usa `CRecord` para `Record<K,V>` o `json.parse`, y
  `CStruct`/`Struct` para shapes.
- **Walker**: el tree-walker no expone la memoria del módulo; los args se
  serializan a un buffer host y los retornos estructurados llegan como `Int`
  crudo (ptr).

Ejemplos: `examples/jit-examples/ffi-struct-real.clsx`,
`examples/jit-examples/ffi-record-typed.clsx`,
`examples/jit-examples/ffi-bump4.clsx`.

## Variables nativas

`var` dentro de `extension` genera funciones **getter/setter** en el scope:
`get_X()` lee y `set_X(valor)` escribe la variable `X`. El backend C del nodo
`clx` no soporta el acceso directo al símbolo de variable y devuelve un error
claro indicando el patrón get/set vía función nativa.

## Estructuras nativas

```clx
extension "libm" as C {
    structure Punto { x: int, y: int };
};
```

Declaran un layout C; luego `Punto(3, 4)` instancia con campos posicionales.

## Kinds de extensión

`extension "<lib>" as <Kind>`: las fórmulas y el parser soportan
`C | Python | Wasm | Js | Wasi | Custom(nombre)`. El backend nativo
**implementado es `C`** (DynamicBackend del nodo: libloading/dlopen/
LoadLibrary); los otros kinds requieren un backend registrado por el nodo
(`Interpreter::set_native_backend` por kind); sin él, el error indica que el
nodo no registró backend para el tipo.

## Límites y mapeo de librerías

- Hasta **4 argumentos** por función nativa (dispatcher `arity0`–`arity4`;
  más de 4 -> error claro).
- Mapeo de nombres de librería por SO:

| Nombre | Windows | Linux | macOS |
|---|---|---|---|
| `libc` / `c` | `msvcrt.dll` | `libc.so.6` | `libSystem.B.dylib` |
| `libm` / `m` | `msvcrt.dll` | `libm.so.6` | `libSystem.B.dylib` |

- Las librerías se cachean por path (no se re-abren por llamada).

## Soporte por intérprete

- **Tree-walker**: sí, vía el backend nativo inyectado por el nodo.
- **JIT**: sí; el emisor compila las llamadas a host functions
  `env.<sym>__<sig>@<lib>` que delegan en el backend del nodo.
- **clxb** (bindings): **no** - backend dummy con error claro
  (`extension nativa 'lib.sym' no soportada por el binding (clxb)`).