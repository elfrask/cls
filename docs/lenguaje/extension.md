# Extensiones nativas (`extension`)

La directiva `extension` declara **símbolos de librerías nativas del sistema**
(`.dll`, `.so`, `.dylib`) para su uso desde CLS: funciones, estructuras y
variables. Es el FFI de CLS.

> Esta es la feature oficial. Detalles de implementación (walker/JIT, backend
> dinámico, ABI) en `agent-context/NATIVE_FFI.md`.

## Sintaxis

```clx
extension "libm" as C {
    function sqrt(x: CDouble) -> CDouble;
    function pow(x: CDouble, y: CDouble) -> CDouble;
};

extension "libc" {          # `as C` es opcional (default)
    function strlen(s: CString) -> CInt;
    structure Punto { x: int, y: int };
    var errno: CInt;        # getters/setters: get_errno() / set_errno(v)
};

# exportar símbolos para módulos importados
extension "libc" as C {
    export function atoi(s: CString) -> CInt;
};
```

- `extension "<librería>" as <Tipo> { ... }` — `<Tipo>` es el **tipo de
  extensión** (backend): `C` hoy; `Python`, `Wasm`, `Js`, `Wasi` en el futuro.
- Las funciones sin cuerpo (`function f(...) -> T;`) son **declaraciones** de
  símbolos nativos.
- `export` hace el símbolo visible en módulos importados.

## Tipos primitivos dedicados (ABI C)

| Tipo CLS | ABI nativo |
|----------|-----------|
| `int` | `i64` |
| `float` | `f64` |
| `bool` | `i32` (0/1) |
| `CString` | `char*` (CLS String → buffer null-terminated) |
| `CPtr<T>` | `void*` / puntero a `T` |
| `CInt` / `CUInt` | `int32_t` / `uint32_t` |
| `CShort` / `CUShort` | `int16_t` / `uint16_t` |
| `CLong` / `CULong` | `long` / `unsigned long` |
| `CChar` / `CUChar` | `int8_t` / `uint8_t` |
| `CFloat` / `CDouble` | `float` / `double` |
| `structure` | puntero a layout C |

> Usa el tipo **exacto** del símbolo C. P. ej. `atoi` devuelve `int` (32 bits)
> → decláralo `-> CInt` (no `-> int`, que es `i64`).

## Resolución

- El símbolo lo resuelve el **nodo** en runtime (`dlopen`/`LoadLibrary`) por
  nombre en la librería declarada — el usuario final solo escribe CLS.
- Nombres canónicos: `libc`, `libm` se mapean al archivo real del SO.
- Para el futuro binario nativo, el linker del SO resuelve en build-time.

## Sandbox

Las librerías nativas rompen el sandbox (acceso total al sistema). El acceso es
**opt-in explícito**: `cls.json → security.allowNative` (o flag CLI). Permitido
por defecto en desktop; bloqueado en contextos no confiables.

## Uso

```clx
extension "libm" as C { function sqrt(x: CDouble) -> CDouble; };
print(sqrt(16.0));   # 4
```

Para declaraciones condicionadas a plataforma, combina con la directiva `when`
(ver `multi-entorno.md`):

```clx
when os: windows {
    extension "user32" as C { function MessageBoxA(h: CPtr, t: CString, c: CString, u: CUInt) -> CInt; }
}
```

## Ver también

- `multi-entorno.md` — la directiva `when` (implementaciones por SO/arquitectura).
- `modulos.md` — `export`/`import` de símbolos entre módulos.
- `agent-context/NATIVE_FFI.md` — plan de implementación y ABI.
