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

Ejemplos: `examples/test/jit-test/extension-demo.clsx`,
`examples/test/jit-test/native-lib.clsx`.

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
| `CStruct` | ptr a la **data** del struct CLS (`base+16`, layout ABI C) |
| `Struct(nombre)` | igual que `CStruct` para la `structure` declarada |
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
struct  : [def_id:i64][n_campos:i64][data con layout ABI C]   (data en +16)
```

Tags runtime de los valores dentro de records: `0=int 1=string 2=float 3=bool
4=char 6=array 7=record`.

- **Arg**: el valor CLS (Record/Array/Struct) se escribe en el layout y el DLL
  recibe su puntero. El wrapper del JIT traduce el offset de la memoria lineal
  del módulo a la dirección host (la memoria lineal es una alocación del host),
  así el DLL lee/escribe el mismo layout en su espacio de direcciones.
  - Los params **struct** (`CStruct`, `Struct(nombre)` o el nombre de la
    `structure` declarada) reciben la dirección de la **data** (`base+16`): el
    DLL ve el struct C con sus offsets desde 0, sin el header CLS. Un `Int`
    crudo (puntero del usuario) viaja tal cual, sin ajuste.
  - Los campos se guardan con su **ancho C** (`i16` -> 2 bytes, `i32` -> 4) y
    alineación natural, así que un `structure` CLS con los mismos tipos que el
    struct C es binario-compatible (`sockaddr_in`, `WSADATA`, ...). Los arrays
    inline `T[n]` (`i8[8]`, `i8[257]`) ocupan su tamaño real — ver
    `estructuras.md`.
- **Retorno**: el DLL escribe el layout y CLS lo vuelve a usar como
  record/array/struct.
  - Si el puntero devuelto cae dentro de la memoria lineal del módulo
    (in-place/zero-copy), se usa el offset directo (cero copias).
  - Si el DLL devuelve un **buffer propio** (malloc, fuera de la memoria del
    módulo), el wrapper lo **re-serializa a la memoria del módulo** re-mapeando
    los punteros internos: las keys de un record y los valores string se
    re-alocan en el módulo (el layout canónico empaqueta punteros de 32 bits, no
    direcciones host de 64 bits). Las keys/valores string del DLL deben usar
    **offsets relativos al buffer** (bajos, < 1MB) en los bits altos:
    `(offset<<32)|len`.
  - `CStruct` como retorno: se declara el `structure` en la `extension` y se
    usa `-> NombreDelStruct`; el wrapper copia el bloque exacto (el tamaño total
    viaja en el nombre del import, `S<bytes>`) y los campos se acceden por
    offsets (`p.x`, `p.y`). Para out-params (el DLL escribe el struct) alcanza
    con declarar el param como `CStruct` y leer los campos después de la llamada
    (`getsockname`).
- **Records con shape** (`var r = {a:1,b:"x"}` inferido) se emiten como struct
  contiguo, no como hashmap: usa `CRecord` para `Record<K,V>` o `json.parse`, y
  `CStruct`/`Struct` para shapes.

Ejemplos: `examples/test/jit-examples/ffi-struct-real.clsx`,
`examples/test/jit-examples/ffi-record-typed.clsx`,
`examples/test/jit-examples/ffi-bump4.clsx`,
`examples/test/jit-test/units/ffi-cstruct.clsx` (bind/listen con
`sockaddr_in`).

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
Los campos usan el layout ABI C (anchos `i8`/`i16`/`i32`/`i64`/`f32`/`f64` y
arrays inline `T[n]`), por lo que la estructura es binario-compatible con el
struct C homónimo al pasarla por FFI. Detalle completo:
`docs/lenguaje/estructuras.md` (sección "Layout en memoria").

## Kinds de extensión

`extension "<lib>" as <Kind>`: las fórmulas y el parser soportan
`C | Python | Wasm | Js | Wasi | Custom(nombre)`. El backend nativo
**implementado es `C`** (DynamicBackend del nodo: libloading/dlopen/
LoadLibrary); los otros kinds requieren un backend registrado por el nodo
(`Interpreter::set_native_backend` por kind); sin él, el error indica que el
nodo no registró backend para el tipo.

Para el patrón `when` + `extension` (selección por SO de la librería
nativa, sin recompilar el core), ver [extension-when.md](extension-when.md).

## Límites y mapeo de librerías

- Hasta **16 argumentos** por función nativa; más de 16 -> error claro
  sugiriendo empaquetar en un struct. La llamada se emite con **libffi** usando
  la firma ABI real por slot (`i64` -> registro entero, `f64` -> registro
  XMM/FP), así que los parámetros `CDouble` llegan correctamente a C en cualquier
  SO/arquitectura. Ver
  [decisión 002](../decisiones/002-ffi-arity-limit.md) para el
  rationale del límite de aridad.
- Mapeo de nombres de librería por SO:

| Nombre | Windows | Linux | macOS |
|---|---|---|---|
| `libc` / `c` | `msvcrt.dll` | `libc.so.6` | `libSystem.B.dylib` |
| `libm` / `m` | `msvcrt.dll` | `libm.so.6` | `libSystem.B.dylib` |

- Las librerías se cachean por path (no se re-abren por llamada).
- **Resolución de la ruta**: un string que no sea un nombre conocido se pasa
  tal cual al cargador (`LoadLibraryExW`/`dlopen`). Una ruta **relativa** se
  resuelve respecto al **directorio de trabajo (CWD)**, no al archivo `.clsx`
  que la declara ni a la carpeta del módulo. Para módulos FFI reutilizables usa
  una **ruta absoluta** o documenta el CWD requerido (limitación conocida; ver
  `examples/brainfuck/README.md`).
- **`CString` de retorno**: **byte-transparente** (los bytes del `char*` se
  copian tal cual a la memoria del módulo, sin normalizar UTF-8 a U+FFFD). Se
  corta en el primer byte `NUL` (semántica de string C, no un bug). Para
  transferir bytes crudos con `NUL` usa `CArray` (ver `bf_read_bytes` del
  oráculo Brainfuck).

## Soporte por intérprete

- **JIT** (`clx run`, `clxr`): sí; el emisor compila las llamadas a host
  functions `env.<sym>__<sig>@<lib>` que delegan en el backend del nodo.
- **clxb** (bindings): **no** - backend dummy con error claro
  (`extension nativa 'lib.sym' no soportada por el binding (clxb)`).