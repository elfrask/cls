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
├── srv_request.clsx   # Request + parseo (query, headers, form/JSON/multipart)
├── srv_response.clsx  # Response + serialización
├── srv_server.clsx    # readRequest / handleRaw / serve
├── validation.clsx    # ValidationResult + validateData + Field/Schema
└── mod.clsx           # barrel
```

Requiere `modules/socket` compilado:

```powershell
powershell -File modules\socket\build.ps1
```

> El FFI de `socket` resuelve la ruta del `.dll` contra la raíz del paquete; ya no depende del CWD.

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
| `req.body` | cuerpo crudo |
| `req.form` | body urlencoded/multipart (`Record<String,String>`) |
| `req.files` | archivos multipart (`FormFile[]`) |
| `req.is_json` | body JSON detectado (acceso vía `req.input`) |
| `req.input(name)` | body/JSON -> query -> params |
| `req.all()` | params + query + form |
| `req.validate(rules)` | valida y devuelve `ValidationResult` |
| `req.validate_schema(schema)` | igual, con reglas construidas con `field()` |
| `req.params`, `req.param(name)` | params de ruta (los rellena el Router) |
| `req.query_param(name)` | query string decodificado |
| `req.file(name)`, `req.has_file(name)` | archivo multipart |
| `parseQuery(qs) -> Record<String,String>` | `a=1&b=2` → `{a:"1", b:"2"}` (percent-decoding) |
| `urlDecode(s) -> String` | decodifica `%XX` y `+` |
| `contentLength(raw) -> int` | `Content-Length` (0 si falta/es inválido) |
| `validateData(data, rules) -> ValidationResult` | motor de reglas pipe |
| `Response(status, body)` | constructor |
| `Response.text/html/json_raw(content)` | 200 + `content-type` |
| `Response.json_response(content)` | alias de `json_raw` (compatibilidad) |
| `Response.redirect(url, code)` | status + `location` |
| `Response.status_only(code)` | status sin body (p. ej. 204/405) |
| `Response.not_found()` | 404 listo |
| `res.header(name, value)` | agrega/sobrescribe un header |
| `res.set_cookie(name, value)` | `Set-Cookie: name=value; Path=/` |
| `res.add_cookie(value)` | agrega un `Set-Cookie` completo (múltiples) |
| `res.serialize() -> String` | status line + headers + body |
| `readRequest(h, timeoutMs) -> String` | lee headers + body completo de un socket |
| `handleRaw(handler, raw) -> String` | procesa un request crudo sin sockets (tests) |
| `serve(host, port, handler) -> int` | loop bloqueante; imprime la URL al escuchar |

## Validación

Dos estilos, sin dependencias:

```clx
# 1) reglas tipo pipe (estilo request de framework)
var v = req.validate({
    name: "required|string|min:2|max:40",
    email: "required|email",
    password: "required|min:8|confirmed"
});
if (v.failed()) { print(v.first("email")); };

# 2) builder chainable
var s = schema({
    name: field().required().string().min(2).to_rule(),
    email: field().required().email().to_rule()
});
var w = req.validate_schema(s);
```

Reglas: `required`, `nullable`, `string`, `int/integer`, `float/numeric`,
`bool/boolean`, `email`, `url`, `date`, `uuid`, `alpha`, `alnum`,
`alpha_dash`, `accepted`, `confirmed`, `min`, `max`, `size`, `between`,
`in`, `not_in`, `same`, `different`, `required_if`, `starts_with`,
`ends_with`, `contains`.

## Body de formularios

- `application/x-www-form-urlencoded` → `req.form` (con `%XX`/`+`).
- `multipart/form-data` → `req.form` (campos) + `req.files` (`FormFile`).
- `application/json` -> `req.is_json` (acceso por `req.input`).

## Limitaciones (v1)

- Solo `Content-Length`: sin `Transfer-Encoding: chunked`.
- Una request por conexión (`Connection: close`); sin keep-alive.
- `headers` es `Record<String, String>`; los headers repetidos van por
  `cookies` (`add_cookie`) para no pisarse. El resto de headers mantiene
  el último valor.
- El loop es bloqueante (sin `async`/`epoll`): suficiente para el MVP.
