# `net` — transporte UDP/TCP por FFI

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §2.2 y §11). Da
acceso a **sockets UDP/TCP** del sistema mediante un shim C (`net_shim.c`) y una
capa CLS idiomática. **No** añade `host_calls`; es FFI puro. Reemplaza al
`host_call http` en su capa de transporte.

## Reglas que cumple

- **Sin punteros expuestos**: los handles de socket viajan como enteros y los
  paquetes como `int[]` (layout `CArray` canónico `[cap:i64][len:i64][elems*8]`,
  un byte por elemento `i64`). **No** se usa `CString` porque los paquetes DNS
  contienen bytes `NUL`.
- **Sin red externa**: pensado para hablar con un servidor local (loopback).

## Build

```powershell
powershell -File modules\net\build.ps1
```

Requiere `gcc` (MinGW-w64 x64) en PATH. En Windows enlaza `ws2_32`. En POSIX
(Linux/macOS) la misma fuente compila a `net_shim.so`/`net_shim.dylib` (la
selección de librería la hace `when` en `net_ffi.clsx`).

> Los shims se resuelven contra la raíz del paquete; ya no depende del CWD.

## API

| Función | Descripción |
|---|---|
| `resolve(host) -> String` | IPv4 de un hostname (`getaddrinfo`); `""` si falla. |
| `udpOpen(ip, port) -> int` | socket UDP `connect`ado a `ip:port` (handle). |
| `udpSend(h, buf: int[]) -> int` | envía los bytes de `buf`. |
| `udpRecv(h, maxLen, timeoutMs) -> int[]` | recibe hasta `maxLen` con timeout (`[]` en timeout). |
| `udpClose(h) -> int` | cierra. |
| `tcpOpen(ip, port) -> int` | conecta TCP. |
| `tcpSend(h, buf) -> int` | escribe `[len:u16 BE][payload]` (framing DNS-over-TCP). |
| `tcpRecv(h, maxLen, timeoutMs) -> int[]` | lee el prefijo y devuelve el payload. |
| `tcpClose(h) -> int` | cierra. |

Clases de estado: `UdpClient` y `TcpClient` (handle encapsulado).

Para conversión de buffers (`int[]`/`Char[]`) usar `modules/bytes`.

Consumidor: `examples/dns/`.
