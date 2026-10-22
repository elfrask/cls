# `httpc` — cliente HTTP/1.1 (CLS puro sobre `socket`)

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §11): cliente HTTP
**escrito desde cero** sobre sockets TCP (FFI), sin `host_calls` y sin el `http`
del nodo. Reemplaza al `host_call http` (deuda §2.7).

> **Nota de nombre**: se llama `httpc` (no `http`) porque `import "http"` colisiona
> con el **módulo interno `http`** del nodo (se resuelve a ese). El transporte vive
> en `modules/socket`.

## Estructura

```
httpc/
├── cls.json
├── url.clsx       # Url: parseo, percent-encoding, resolve/redirects
├── headers.clsx   # Headers: case-insensitive, ordenada, duplicados (Set-Cookie)
├── body.clsx      # jsonBody / formBody / Multipart
├── cookies.clsx   # cookie jar por host
├── request.clsx   # Request + serialización
├── response.clsx  # Response (+ .text() / .json() / .header())
├── stream.clsx    # lectura de cuerpo (Content-Length / chunked / EOF)
├── transport.clsx # intercambio request/response sobre un socket
├── session.clsx   # Session (keep-alive + redirects + cookies) + atajos
└── mod.clsx       # barrel
```

## Requisitos

Compilar el shim del transporte y correr desde la raíz del repo:

```powershell
powershell -File modules\socket\build.ps1
```

```json
{ "pathModules": ["../../modules"] }
```

## Uso

```clx
import "httpc" as http;

function main(args: String[]) -> int {
    # atajo sin estado
    var r = http::get("http://example.com/");
    print(str(r.status));
    print(r.text());

    # sesión con keep-alive, redirects y cookies
    var s = http::Session();
    var j = s.get("http://example.com/api").json();
    print(str(j["ok"]));
    s.post("http://example.com/api", http::jsonBody({ n: 1 }), "application/json");
    s.close();
    return 0;
};
```

## API

| Símbolo | Descripción |
|---|---|
| `get` / `head` / `del` / `post` / `put` / `patch` | atajos de un solo uso |
| `Session` | keep-alive (pool por host:puerto) + redirects + cookie jar |
| `Request` / `Response` | `Response`: `.status`, `.header(n)`, `.text()`, `.json()`, `.ok()` |
| `Headers` | `add/get/getAll/set/remove/has` |
| `Url` / `parseUrl` | parseo + `percentEncode/Decode`, `encodeQuery` |
| `jsonBody` / `formBody` / `Multipart` | codificadores de cuerpo |
| `Cookies` | jar simple por host |

**Soporta**: métodos GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS, cabeceras,
`Content-Length`, `Transfer-Encoding: chunked` (con trailers), redirects
(301/302/303/307/308, hasta `maxRedirects`), **keep-alive** (reutiliza la
conexión por host:puerto), cookies (`Set-Cookie`/`Cookie`), timeouts,
`form-urlencoded`, JSON y `multipart/form-data`.

## Limitaciones

- **Solo `http://`**. HTTPS/TLS requiere un shim TLS (pendiente): el transporte es
  `socket` (TCP crudo).
- Sin `Content-Encoding` (gzip/deflate): no se anuncia `Accept-Encoding`.
- Sin HTTP/2; sin `userinfo@` en URLs (se documenta/acota).

## Limitaciones conocidas (codegen del lenguaje)

Encontradas al construir el módulo (candidatas a bugfix de dev-3):

- **Llamar a una función de OTRO módulo omitiendo un default** emite WASM
  inválido → `socket::line(h, ms, 65536)` pasa `maxLen` explícito.
- **`stem` de archivo == nombre de símbolo importado** (p. ej. `exchange.clsx` con
  `export function exchange`) emite WASM inválido → el archivo se llama
  `transport.clsx`.
- **Defaults en métodos de clase** → WASM inválido (ver `modules/socket`).
- **Métodos que devuelven `int[]` desde FFI** necesitan temporal anotado.
- **`from` / `method` son palabras reservadas** (no usables como identificadores).
