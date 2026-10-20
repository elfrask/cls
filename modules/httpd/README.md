# `httpd` — servidor HTTP/1.1 (CLS puro sobre `socket`)

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §2.2 y §11): parser
de request, builder de response y loop `accept → read → handler → write` para el
lado **servidor**. El transporte es FFI (`modules/socket`); este módulo no usa
`host_calls`.

- **CLS puro**: no expone punteros ni requiere una DLL propia.
- **Reutilizable**: el framework web `tombstone` lo usa como capa HTTP.
- El nombre es `httpd` (no `http`) porque `import "http"` colisiona con el
  módulo interno `http` del nodo.

## Estructura

```
httpd/
├── cls.json
├── srv_status.clsx    # razones HTTP + helpers
├── srv_request.clsx   # Request + parseo (query, headers, body)
├── srv_response.clsx  # Response + serialización
├── srv_server.clsx    # readRequest / handleRaw / serve
└── mod.clsx           # barrel
```

Requiere `modules/socket` compilado:

```powershell
powershell -File modules\socket\build.ps1
```

> El FFI de `socket` resuelve la ruta del `.dll` relativa al **CWD**: para el
> servidor hay que correr desde la raíz del repo CLS.

## Uso

```clx
from "httpd" import Request, Response, serve;

function main(args: String[]) -> int {
    return serve("127.0.0.1", 8080, (req: Request) -> Response {
        if (req.path == "/") {
            return Response.html("<h1>hola</h1>");
        };
        return Response.not_found();
    });
};
```

```powershell
curl.exe -i http://127.0.0.1:8080/
```

## API

| Símbolo | Descripción |
|---|---|
| `Request.parse(raw: String) -> Request` | parsea request-line + headers + body |
| `req.http_method`, `req.path`, `req.query`, `req.query_string`, `req.version` | línea de petición |
| `req.headers`, `req.header(name)` | headers normalizados a minúsculas |
| `req.body` | cuerpo tal cual (sin decodificar) |
| `req.params`, `req.param(name)` | params de ruta (los rellena el Router) |
| `req.query_param(name)` | query string decodificado a record (sin `%`) |
| `parseQuery(qs) -> Record<String,String>` | `a=1&b=2` → `{a:"1", b:"2"}` |
| `contentLength(raw) -> int` | `Content-Length` (0 si falta/es inválido) |
| `Response(status, body)` | constructor |
| `Response.text/html/json_raw(content)` | 200 + `content-type` |
| `Response.json_response(content)` | alias de `json_raw` (compatibilidad) |
| `Response.redirect(url, code)` | status + `location` |
| `Response.status_only(code)` | status sin body (p. ej. 204/405) |
| `Response.not_found()` | 404 listo |
| `res.header(name, value)` | agrega/sobrescribe un header |
| `res.set_cookie(name, value)` | `Set-Cookie: name=value; Path=/` |
| `res.serialize() -> String` | status line + headers + body |
| `readRequest(h, timeoutMs) -> String` | lee headers + body completo de un socket |
| `handleRaw(handler, raw) -> String` | procesa un request crudo sin sockets (tests) |
| `serve(host, port, handler) -> int` | loop bloqueante; imprime la URL al escuchar |

## Limitaciones (v1)

- Solo `Content-Length`: sin `Transfer-Encoding: chunked`.
- Una request por conexión (`Connection: close`); sin keep-alive.
- `headers` es `Record<String, String>`: el último valor gana. `Set-Cookie`
  múltiple queda para una fase posterior.
- Query sin percent-decoding (`%20`, `+`); se agrega junto con `modules/url` si
  hace falta.
- El loop es bloqueante (sin `async`/`epoll`): suficiente para el MVP.
- El body se entrega crudo (sin `application/x-www-form-urlencoded` ni
  `multipart`).
