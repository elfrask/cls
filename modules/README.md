# `/modules` — módulos compartidos de CLS

Módulos **CLS puro** reutilizables entre los proyectos de prueba de `examples/`.
Son los ladrillos de la **stdlib autocontenida**: se apoyan solo en
intrínsecos primitivos, `print`/`input` y **FFI a C** (ver
`examples/patrones.md` §2 y §11).

## Reglas

1. **CLS puro + FFI.** Nada de `host_calls` ni intrínsecos nuevos. Si falta una
   capacidad, se resuelve con FFI (`extension`), no desde el nodo.
2. **Sin dependencias externas** fuera de CLS + FFI.
3. **Un módulo = una carpeta** en `modules/<nombre>/` con su `cls.json`.
4. **No copiar/pegar** módulos entre proyectos: se referencian (ver abajo).
5. Si un módulo reemplaza a un `host_call` (`fs`, `http`, `json`, `strings`),
   se anota en la tabla de deuda de `examples/patrones.md` §2.7.

## Estructura de un módulo

Un módulo es una carpeta con su `cls.json`. `export.entry` apunta al **barrel**
`mod.clsx`, que reexporta la API pública desde archivos enfocados (una
responsabilidad por archivo):

```
modules/<nombre>/
├── cls.json          # { "name": "...", "version": "...", "export": { "entry": "mod.clsx" } }
├── mod.clsx          # barrel: `import "x" as x; export { x::sym as sym };`
├── <dominio>.clsx    # ... un archivo por responsabilidad ...
└── <mod>_ffi.clsx    # bindings FFI crudos (no públicos), si el módulo usa FFI
```

- Los imports internos (`from "dominio" import ...`) se resuelven desde la
  **raíz del módulo** (aislamiento; requiere `cls.json` con `export.entry`).
- Los helpers internos se `export`an entre archivos pero **no** se reexportan en
  el barrel: no son API pública.
- **Gotcha**: el typeck registra los módulos internos por el **stem** del
  archivo (`"dominio"`), así que dos archivos `.clsx` con el **mismo nombre** en
  módulos distintos del grafo **colisionan**. Usá stems únicos
  (p. ej. `io_ffi.clsx` / `net_ffi.clsx`, no `ffi.clsx` en ambos).
- Las reexportaciones de **tipos** conservan el nombre original
  (`x::Clase as Clase`); funciones y constantes admiten renombrado.

## Cómo consumirlo desde un proyecto

Como `modules/` está fuera de cada proyecto (`examples/<prueba>/`), hay que
indicar la ruta de búsqueda con **una** de estas dos formas (o ambas):

**Flag de CLI** (relativa al CWD):

```bash
clx run examples/brainfuck/src/main.clsx --add-path-module=modules
clx run app.clsx --add-path-module=modules --add-path-module=libs
```

**`cls.json` del proyecto** (`pathModules`, relativa a la raíz del proyecto):

```json
{
  "name": "brainfuck",
  "entry": "src/main.clsx",
  "pathModules": ["../../modules"]
}
```

Luego, en el código:

```clx
from "str" import *;                 # usa modules/str/
import "sha256" as sha;              # usa modules/sha256/  -> sha::hash(...)
```

## Módulos

| Módulo | Estado | Reemplaza a | Notas |
|--------|--------|-------------|-------|
| `io` | Activo | `host_call fs` (§2.7) | **FFI a C**. I/O de archivos y descriptores sobre el shim `io_shim.dll` (sin punteros). Pagan la deuda de autocontención. Requiere `powershell -File modules/io/build.ps1`. Usado por `examples/brainfuck` y `examples/clsh`. |
| `bytes` | Activo | `bf_ord`/`bf_read_bytes` de brainfuck | **CLS puro (sin FFI)**. `ord`/`chr` (sobre los intrínsecos `char`/`ord`; `Char` = unidad de 1 byte) + `hexEncode`/`hexDecode`/`firstDiff`. Sin build. Usado por `examples/brainfuck`. |
| `prng` | Activo | `host_call math.random`; LCG local de `conformance` | CLS puro. `Prng` (LCG de 64 bits): `next`, `nextInt`, `nextFloat`. Usado por `examples/conformance`. |
| `check` | Activo | — | CLS puro. Aserciones + protocolo `CHECK k=v` (`assertEq`, `assertEqStr`, `assertTrue`, `fail`, `fails`, `reset`) para runners de prueba. |
| `config` | Activo | — | CLS puro. Introspección de la configuración del host **solo lectura**: `all`, `get`, `getBool`, `getInt`, `sandboxEnabled`. Envuelve el intrinsic `config()` con caché perezosa (una sola llamada al host). |
| `orm` | Activo | — | CLS puro. Capa ORM **agnóstica al motor** + contrato de driver (`Cell`, `ResultSet`, `Driver` base) y `Orm` (CRUD, query builder, transacciones, bulk). Backends enchufables. Extraído de `examples/sqlite-orm`. |
| `sqlite` | Activo | — | **FFI a C**. Backend SQLite (`SqliteDriver: Driver`) sobre el shim `sqlite_shim.dll` (sin punteros; handles enteros). Consume `orm`. Requiere `powershell -File modules/sqlite/build.ps1`. |
| `net` | Activo | `host_call http` (capa de transporte, §2.7) | **FFI a C**. Sockets UDP/TCP sobre el shim `net_shim.dll` (sin punteros; handles enteros; payloads como `int[]`). `udpOpen/Send/Recv/Close`, `tcpOpen/Send/Recv/Close` (framing DNS-over-TCP), `resolve`, clases `UdpClient`/`TcpClient`. Requiere `powershell -File modules/net/build.ps1`. Usado por `examples/dns`. |
| `dns` | Activo | — | **CLS puro (sin FFI)**. Codec DNS RFC 1035: big-endian, nombres con compresión `0xC0` + guard anti-loop, RRs A/AAAA/CNAME/NS/MX/TXT/SOA, `buildQuery` y `answer(query, zone)` (autoritativo). Consume `net` para transporte. Usado por `examples/dns`. |
| `socket` | Activo | `host_call http` (transporte) | **FFI a C**. Sockets TCP **persistentes y bidireccionales** (keep-alive/streaming) sobre `socket_shim.dll` (handles enteros; bytes como `int[]`). `connect/sendAll/sendText/recv/recvExact/line/readable/setTimeout/shutdownWrite/close` + clase `Client`. Requiere `powershell -File modules/socket/build.ps1`. Base de `httpc`. |
| `httpc` | Activo | `host_call http` | **CLS puro sobre `socket`**. Cliente **HTTP/1.1** completo: métodos GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS, cabeceras, `Content-Length`/`chunked`, **redirects**, **keep-alive**, **cookies**, JSON/form/multipart; `Session` (pool + redirects + cookies) y atajos `get/post/...`, `Request`/`Response` (`.text()`/`.json()`). Solo `http://` (sin TLS). Se llama `httpc` porque `http` colisiona con el interno del nodo. |

## Excepción sancionada: `config()`

`config` es un **intrinsic de introspección autorizado** (excepción explícita a
la regla 5.1.6 de no tocar el host): no añade una *feature* del lenguaje, solo
**lee parámetros del host** (`cls.json` + flags de CLI) y los expone como un
`Record<String, String>` plano de solo lectura. El módulo `config/` es la capa
CLS idiomática sobre él (caché perezosa + helpers tipados). Ver
`docs/guia/configuracion.md`.

