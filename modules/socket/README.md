# `socket` — sockets TCP persistentes (FFI a C)

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §2.2 y §11). Expone
un **stream TCP crudo y bidireccional** sobre el shim `socket_shim.dll`. A
diferencia de `modules/net` (que aplica el framing DNS-over-TCP), la conexión
queda **abierta** y el llamador envía/recibe cuantas veces quiera (HTTP
keep-alive, WebSocket, protocolos propios).

- **CLS + FFI**, sin `host_calls`, sin punteros (handles enteros; bytes como
  `int[]`).
- Reemplaza al `host_call http` en su capa de transporte (deuda §2.7).

## Estructura

```
socket/
├── cls.json
├── socket_shim.c      # FFI crudo (ws2_32/libc)
├── socket_ffi.clsx    # bindings (no público)
├── client.clsx        # API funcional + clase `Client`
├── mod.clsx           # barrel
└── build.ps1          # compila socket_shim.dll
```

## Build

```powershell
powershell -File modules\socket\build.ps1   # requiere gcc (MinGW-w64 x64)
```

> El FFI resuelve la ruta del `.dll` relativa al **CWD**: correr desde la raíz del
> repo.

## API

Funcional:

| Símbolo | Descripción |
|---|---|
| `connect(host, port) -> int` | conecta; handle `>=0` o `<0` error |
| `connectTimeout(host, port, ms) -> int` | idem con timeout (`-4` = timeout) |
| `sendAll(h, buf: int[]) -> int` | envía TODOS los bytes |
| `sendText(h, s: String) -> int` | envía texto (como bytes) |
| `recv(h, maxLen, ms) -> int[]` | recibe hasta maxLen (`[]` en timeout/EOF) |
| `recvExact(h, n, ms) -> int[]` | lee exactamente n bytes (`[]` si no) |
| `recvText(h, maxLen, ms) -> String` | recibe como texto |
| `line(h, ms, maxLen=65536) -> String` | lee una línea (sin `\n`) |
| `readable(h, ms) -> int` | 1 si hay datos (o EOF) listos |
| `setTimeout(h, ms) -> int` | timeout de E/S a nivel socket |
| `shutdownWrite(h) -> int` | half-close (EOF al peer) |
| `close(h) -> int` | cierra |
| `lastError() -> String` | último error del sistema |

> Conversión de buffers: usar los intrínsecos `String.toChar()` / `String.toBytes()`
> y `modules/bytes` (`toChars`/`toInts`/`toStr`).

OO:

```clx
import "socket" as socket;

function main(args: String[]) -> int {
    var c = socket::Client("127.0.0.1", 8080);
    if (!c.ok()) { return 1; }
    c.writeText("GET / HTTP/1.1\r\n\r\n");
    print(c.readLine());          # primera línea de estado
    c.close();
    return 0;
};
```

`Client`: `ok`, `handle`, `write`, `writeText`, `read`, `readText`, `readExact`,
`readLine`, `readableNow`, `waitReadable(ms)`, `setTimeout(ms)`, `shutdownWrite`,
`close`.

## Limitaciones conocidas (codegen / FFI)

- **Defaults en métodos de clase** hoy emiten WASM inválido → `Client` no usa
  valores por defecto de parámetro (el timeout es un campo).
- Un **método que devuelve `int[]` desde un `CArray` de FFI** debe asignarlo
  primero a un temporal `int[]` anotado (`read`/`readExact` lo hacen).
- Solo **TCP**; TLS corre por encima (ver `modules/httpc`).
