# `when` + `extension` — bindings nativos portables por SO

> Guía canónica del patrón que combina la directiva compile-time `when`
> con la declaración FFI `extension` para escribir bindings nativos del
> sistema operativo **sin recompilar cls-core/cls-jit**.

## Regla de oro

**`when` envuelve a `extension`. Nunca al revés.**

- `extension` declara **firmas puras** (sin cuerpo, terminan en `;`).
- Las firmas nativas no tienen bloque de código, así que no hay
  dónde poner lógica condicional adentro.
- `when` es una directiva compile-time que selecciona qué código
  generar según el target. Como `extension` no genera código (solo
  registra imports), `when` tiene que vivir afuera.

El parser lo hace cumplir: dentro de `extension` solo se permiten
`function`, `structure` y `var` (ver `cls-core/src/frontend/parser.rs:1364-1368`).
Cualquier otra cosa — incluyendo `when` — es un error de sintaxis.

## Patrón canónico: sockets TCP portables

El caso más común: hacer un binding a la API de sockets del SO.
Cada SO tiene su librería y a veces nombres diferentes. La receta:

```clx
# modulo de sockets portable
# (extracto; el modulo completo vive en examples/audit/test-features/extension-demo.clsx)

when (os: windows) {
    extension "ws2_32.dll" as C {
        function socket(af: CInt, type: CInt, proto: CInt) -> CInt;
        function bind(sock: CInt, addr: CPtr, len: CInt) -> CInt;
        function listen(sock: CInt, backlog: CInt) -> CInt;
        function accept(sock: CInt, addr: CPtr, len: CPtr) -> CInt;
        function recv(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function send(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function closesocket(sock: CInt) -> CInt;
        function WSAGetLastError() -> CInt;
    };
}

when (os: linux) {
    extension "libc.so.6" as C {
        function socket(af: CInt, type: CInt, proto: CInt) -> CInt;
        function bind(sock: CInt, addr: CPtr, len: CInt) -> CInt;
        function listen(sock: CInt, backlog: CInt) -> CInt;
        function accept(sock: CInt, addr: CPtr, len: CPtr) -> CInt;
        function recv(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function send(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function close(fd: CInt) -> CInt;
    };
}

when (os: macos) {
    extension "libSystem.B.dylib" as C {
        function socket(af: CInt, type: CInt, proto: CInt) -> CInt;
        function bind(sock: CInt, addr: CPtr, len: CInt) -> CInt;
        function listen(sock: CInt, backlog: CInt) -> CInt;
        function accept(sock: CInt, addr: CPtr, len: CPtr) -> CInt;
        function recv(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function send(sock: CInt, buf: CPtr, len: CInt, flags: CInt) -> CInt;
        function close(fd: CInt) -> CInt;
    };
}

# Capa de azucar portable: la API que el resto del codigo ve.
# Esta capa ES CLS puro (no es FFI), asi es uniforme en todos los SOs.
function net_listen(port: Int) -> Int {
    var sock = socket(2, 1, 0);   # AF_INET, SOCK_STREAM, 0
    if sock < 0 { return 0; }
    # ... bind a 127.0.0.1:port + listen ...
    return sock;
}

# net_accept, net_recv, net_send, net_close: idem, sobre las extension de arriba.
```

Solo **una** de las tres ramas de `when` se emite al WASM final. En
runtime, cero costo para las inactivas.

## Tabla de equivalencias por SO

| API                      | Windows         | Linux         | macOS              |
|--------------------------|-----------------|---------------|--------------------|
| libc general             | `msvcrt.dll`    | `libc.so.6`   | `libSystem.B.dylib`|
| libm (math)              | `msvcrt.dll`    | `libm.so.6`   | `libSystem.B.dylib`|
| sockets (BSD)            | `ws2_32.dll`    | `libc.so.6`   | `libSystem.B.dylib`|
| cerrar socket            | `closesocket`   | `close`       | `close`            |
| ultimo error             | `WSAGetLastError` | `errno` (via `__errno_location`) | `errno` (idem) |

**Notas**:
- En macOS, `socket/recv/send/close` viven en `libSystem.B.dylib` (que
  es un superset de libc + libm + extras). Por eso el bloque de macOS
  es casi identico al de Linux — solo cambia la libreria.
- En Windows, Winsock2 (`ws2_32.dll`) tiene su propio set de errores
  via `WSAGetLastError()`, separado del `errno` de MSVCRT.
- Para libc en Windows, MSVCRT expone la mayoria de POSIX pero **no
  sockets** — esos requieren Winsock2.

## Combinadores de `when`

`when` acepta `and`, `or`, `not` y parentesis para condiciones
complejas. Ejemplos utiles:

```clx
# Aplica a cualquier Unix-like (Linux, macOS, BSD).
when (os: linux) or (os: macos) or (os: freebsd) {
    extension "libc.so.6" as C {
        function getpid() -> CInt;
    };
}

# Aplica solo a ARM de 64 bits (servidor, movil, Apple Silicon).
when (arch: arm64) {
    extension "..." as C {
        # instrucciones SIMD especificas de ARM64
    };
}

# Bare-metal: ningun SO, solo arquitectura.
when (os: none) and (arch: riscv64) {
    # codigo embebido
};
```

Referencia completa: ver [multi-entorno.md](multi-entorno.md).

## Anti-patrones

### `when` dentro de `extension`

```clx
# MAL: el parser rechaza con error de sintaxis
extension "ws2_32.dll" as C {
    when (os: windows) {       # <- "En extension solo se permiten
        function socket(...); #    declaraciones function, structure o var"
    };
};
```

### `extension` con cuerpo

```clx
# MAL: extension es firma pura, no implementacion
extension "libc.so.6" as C {
    function strlen(s: CString) -> CInt {
        return 0;   # <- esto no compila
    };
};
```

Si queres una capa portable sobre las firmas nativas, **ponela afuera**:

```clx
# BIEN: la capa de azucar es CLS puro, vive al lado
extension "libc.so.6" as C {
    function strlen(s: CString) -> CInt;
};

function mi_strlen(s: String) -> Int {
    return strlen(s);
}
```

### Asumir que `when` se evalua en runtime

`when` es compile-time. Las ramas inactivas **no existen en el WASM
emitido**. Si queres logica que cambia en runtime, usa `if`.

### Multiples `extension` con el mismo simbolo para el mismo target

```clx
# MAL: dos declaraciones del mismo simbolo en el mismo target
when (os: linux) {
    extension "libc.so.6" as C {
        function strlen(s: CString) -> CInt;
    };
    extension "msvcrt.dll" as C {       # <- ambiguo
        function strlen(s: CString) -> CInt;
    };
}
```

El emisor duplica el import. El linker del SO resuelve (coge uno), pero
el comportamiento es indefinido y varia por SO.

## Como migrar una feature que antes era interna

Caso de estudio: el modulo `net` (sockets TCP) que existia en cls-jit
como hosts nativos. Eliminado en dev-2 (commit `157127a`).

La receta para migrar cualquier feature similar:

1. **Identificar los hosts** en `cls-jit/src/host.rs` (eran
   `host_net_listen/accept/recv/send/close`).
2. **Identificar la signatura WASM** en
   `cls-core/src/backend/wasm/host_fn.rs` (eran los enum variants
   `NetListen/Accept/Recv/Send/Close`).
3. **Identificar el dispatch del emisor** en
   `cls-core/src/backend/wasm/emitter/module_calls.rs` (era
   `emit_net_call`).
4. **No** traducir a una "libreria interna" nueva en cls-jit.
5. **Si** traducir a firmas `extension` con `when` por SO en un
   `.clsx` del usuario.
6. Crear la capa de azucar portable (funciones en CLS puro que
   envuelven las extension) en ese mismo `.clsx`.
7. Eliminar los hosts, los enum variants y el dispatch del emisor.

**Resultado**: la feature pasa de ~200 lineas en 3 archivos del
runtime a un `.clsx` de ~100 lineas que el usuario puede modificar
**sin recompilar el core**. Ese es el objetivo arquitectural.

## Cuando SI se necesita un host en el runtime

Hay 3 casos donde `extension`+`when` no alcanza y se requiere codigo
nativo en cls-jit:

1. **Async I/O no bloqueante** (epoll/kqueue/IOCP): requiere suspender
   la corrutina mientras espera un evento. El lenguaje todavia no tiene
   async en el JIT (ver `agent-context/ASYNC_PLAN.md`).
2. **Tipos con layout opaco** (file descriptors, `pthread_t`, handles
   del SO): el dispatcher de FFI no puede representarlos limpiamente
   en tipos CLS. Workaround: typedefs via `Int` + wrapping manual.
3. **Buffers muy grandes** (read/write de GBs): cero-copy requiere
   acceso directo a la memoria lineal del modulo. El wrapper del JIT
   ya lo hace para `CArray`/`CRecord`/`CStruct`
   (ver [extension.md](extension.md) seccion "Valores estructurados").

## Ver tambien

- [extension.md](extension.md) — referencia completa de kinds, tipos C,
  mapeo de nombres, valores estructurados.
- [multi-entorno.md](multi-entorno.md) — referencia completa de `when`:
  prefijos, combinadores, simulacion con `--target`, comportamiento
  por interprete.
- [modulos.md](modulos.md) — sistema de modulos fuente de CLS
  (distinto del FFI: cubre `import`/`from`/`include`).
- [decisiones/001-no-implicit-coercion.md](../decisiones/001-no-implicit-coercion.md) —
  ejemplo de como se estructura una decision de diseno en este repo.
- `examples/audit/test-features/extension-demo.clsx` — ejemplo
  funcional real de `extension` con `libm` y `msvcrt.dll`.
- `examples/audit/test-features/jit-test/units/a10.clsx` — ejemplo
  de `when` con `os:`, `arch:`, `cls-arch`.
