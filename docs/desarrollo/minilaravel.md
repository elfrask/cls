# Minilaravel — Framework HTTP en CLS puro (especificación para implementar)

> **Estado**: especificación lista para delegar. La implementación la hace un
> desarrollador asignado. Este documento es auto-contenido: el dev NO necesita
> leer el código del proyecto para implementar; solo consultar los docs
> referenciados.
>
> **Requisito duro**: 100% `.clsx` puro. NO se toca el host (`cls-core`,
> `cls-jit`, `cls-internals`, `nodos/clx/src/modules/`). Si una capacidad no es
> expresable con `extension` + `when` + CLS puro, se documenta como limitación
> — no se agrega código nativo.

## Entregables

1. `modules/socket.clsx` — capa de sockets TCP portables (F1)
2. `modules/http.clsx` — parser HTTP + clases `Request`/`Response` (F2)
3. `modules/router.clsx` — enrutador con params (F3)
4. `modules/middleware.clsx` — cadena de middleware (F4)
5. `modules/static.clsx` — archivos estáticos (F5)
6. `modules/view.clsx` — renderer CMX → HTML (CLS puro) (F6)
7. `examples/http-framework/main.clsx` — demo completa (F7)
8. `examples/http-framework/views.clsx` — vistas de la demo (funciones CLS que devuelven CMX)
9. `examples/http-framework/README.md` — cómo correr y probar

## Docs de referencia (leer primero)

| Tema | Doc |
|---|---|
| FFI `extension` (tipos C, aridad 16, structs) | `docs/lenguaje/extension.md` |
| Patrón `when` + `extension` (sockets TCP por SO) | `docs/lenguaje/extension-when.md` |
| Directiva `when` (combinadores, target) | `docs/lenguaje/multi-entorno.md` |
| Clases/OOP (ctor `main`, `me.`, `super`) | `docs/lenguaje/oop.md` |
| Records/arrays/strings | `docs/lenguaje/datos.md` |
| Módulos (`import`/`from`/`include`) | `docs/lenguaje/modulos.md` |
| Estado real del runtime (WASI NO, async NO) | `docs/lenguaje/estado-runtime.md` |
| **CMX (sintaxis de vistas)** | `docs/lenguaje/cmx.md` + tests `b8-cmx.clsx`, `10-cmx.clsx` |
| Módulo `fs` (para static) y `json` | `docs/stdlib/desktop.md`, `docs/stdlib/core.md` |
| Ejemplos funcionales de FFI | `examples/audit/test-features/extension-demo.clsx`, `examples/audit/test-features/tests/test-ffi-arity.clsx` |

## Restricciones del lenguaje (NO negociables)

1. **Comentarios con `#`**, no `//` (rompe el lexer). Nada de arrows no-ASCII `→` en `.clsx`.
2. **Strings inmutables**: `s = s.upper()`, no `s.upper()` a secas.
3. **Concatenación**: `"texto" + int` NO compila — usar `.toString()`.
4. **Ctor de clase**: `function main(...)`, no `__init`.
5. **Handlers con tipos explícitos**: `(req: Request) -> Response`, nunca params bare (el typeck infiere `Any` y falla el dispatch — ver `docs/lenguaje/magics.md`).
6. **`when` envuelve `extension`, nunca al revés**.
7. **Sin async/await**: el servidor es bloqueante por request.
8. **Las vistas usan CMX** (la sintaxis nativa de maquetación). NO se inventa un
   motor de templates (nada de `{{ }}`, `@if`, `@for`, Blade). El renderer
   CMX→HTML es CLS puro (módulo `render.clsx`), ver F6.

---

## F1. Capa de sockets TCP (`modules/socket.clsx`)

Patrón exacto de `docs/lenguaje/extension-when.md` (sección "Patrón canónico: sockets TCP portables"). Declarar por SO:

```clx
# Windows
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
        function htons(hostshort: CInt) -> CInt;
        function inet_addr(cp: CString) -> CInt;
    };
}
# Linux: igual con "libc.so.6", close(fd) en vez de closesocket, socket/recv/send
# macOS: igual con "libSystem.B.dylib", close(fd)
```

Capa de azúcar portable (CLS puro, la API que el resto ve):

```clx
class Socket {
    var fd: int;
    function main(fd: int) { me.fd = fd; }
    static function listen(port: int) -> Socket {
        # socket(AF_INET=2, SOCK_STREAM=1, 0)
        # bind a INADDR_ANY (0.0.0.0) con sockaddr_in (16 bytes: fam=2, port, addr)
        # listen(fd, backlog=16)
        # devuelve Socket(fd) o null/0 si error
    }
    function accept() -> Socket { ... }      # accept(fd) -> nuevo Socket
    function recv(max: int) -> String { ... } # recv(fd, buf, max, 0) -> bytes
    function send(data: String) -> int { ... } # send(fd, data, len, 0)
    function close() -> int { ... }
};
```

**Notas**:
- En Windows hay que llamar `WSAStartup` una vez antes de `socket` (declarar en la extension e invocar al iniciar).
- `sockaddr_in` se construye en memoria lineal: alocar 16 bytes, escribir `family=2` (i16), `port` (i16 en network byte order vía `htons`), `addr` (i32 vía `inet_addr("0.0.0.0")`). Usar `CStruct`/`structure` de `extension.md` si facilita el layout.
- Aridad: todas las llamadas ≤ 16 args (decisión 002). Si alguna excede, empaquetar en struct.
- El socket fd viaja como `int` (handle opaco — ver "tipos con layout opaco" en extension-when.md).

## F2. Parser HTTP (`modules/http.clsx`)

Clases:

```clx
class Request {
    var method: String;      # GET/POST/PUT/DELETE
    var path: String;        # /users/42
    var query: Record<String, String>;  # ?a=1&b=2
    var headers: Record<String, String>; # case-insensitive (guardar en minuscula)
    var body: String;        # por Content-Length
    function main(raw: String) { ... }   # parsea el buffer crudo
    static function parse(raw: String) -> Request { ... }
};

class Response {
    var status: int;
    var headers: Record<String, String>;
    var body: String;
    function main(status: int, body: String, headers: Record<String, String>) { ... }
    static function json(obj: any) -> Response { ... }   # body = json.stringify(obj), header Content-Type: application/json
    static function text(s: String) -> Response { ... }
    static function html(s: String) -> Response { ... }  # Content-Type: text/html
    static function status(n: int) -> Response { ... }   # sin body
    function header(name: String, value: String) -> Response { ... }
};
```

**Parseo** (CLS puro, usar `str.split`, `str.indexOf`, `str.slice` del módulo strings o los métodos de String):
1. Dividir el buffer por `\r\n\r\n` → headers vs body.
2. Request line: `METHOD SP PATH SP HTTP/1.1`.
3. Path + query: separar en `?`.
4. Headers: cada línea `Nombre: valor`, normalizar nombre a minúsculas.
5. Body: leer `Content-Length` bytes del resto.
6. **Chunked encoding**: documentar como limitación v1 (o implementar si queda tiempo — `Transfer-Encoding: chunked`).

**Serialización** (para enviar):
```clx
function serialize_response(res: Response) -> String {
    # "HTTP/1.1 200 OK\r\nContent-Type: ...\r\nContent-Length: N\r\n\r\n" + body
}
```

## F3. Router (`modules/router.clsx`)

```clx
class Router {
    var routes: Array;  # [{method, pattern, handler}]
    function main() { ... }
    function get(pattern: String, handler: (Request) -> Response) { ... }
    function post(pattern: String, handler: (Request) -> Response) { ... }
    function put(pattern: String, handler: (Request) -> Response) { ... }
    function delete(pattern: String, handler: (Request) -> Response) { ... }
    function match(method: String, path: String, req: Request) -> Response {
        # recorre routes; si method+pattern coinciden, extrae params en req y llama handler(req)
        # si no: 404 (o 405 si el path existe con otro metodo)
    }
};
```

- Patrones: `/users/{id}` → params `{id: "42"}` en `req.query` o un campo `req.params: Record<String, String>`.
- Coincidencia: split del path por `/` y comparar segmento a segmento; `{x}` matchea cualquier valor.
- Handlers: tipo `(Request) -> Response` (función). Si un handler es una arrow `(req) => ...`, anotar el param `(req: Request) -> Response` (restricción 5).
- 404 por defecto: `Response.status(404).text("Not Found")`.

## F4. Middleware (`modules/middleware.clsx`)

```clx
class App {
    var router: Router;
    var middlewares: Array;   # [(Request) -> Request|Response|...]
    var static_dirs: Array;
    function main() { ... }
    function use(fn: (Request) -> Request) { ... }   # registra middleware
    function get/post/put/delete(pattern, handler) { me.router.get(...); }
    function static(prefix: String, dir: String) { ... }
    function handle(raw: String) -> Response {
        # 1. parsea Request
        # 2. corre middlewares en cadena (cada uno puede mutar req o cortar con Response)
        # 3. static si el path matchea un prefijo
        # 4. router.match
    }
    function serve(port: int) {
        # loop: socket.listen(port); accept(); recv() -> handle() -> send(); close()
        # bloqueante; log a consola
    }
};
```

- Middleware que corta: devolver un `Response` en vez de `Request` (convención: si el retorno es Response, se envía y se detiene la cadena).
- Ejemplos: logging (imprimir método+path), CORS (`Access-Control-Allow-Origin: *`), parseo de body JSON.

## F5. Archivos estáticos (`modules/static.clsx`)

- `app.static("/public", "./public")`: si `req.path` empieza con `/public/`, leer el archivo de `./public/` + el resto del path.
- Usar el módulo desktop `fs` (`import "fs" as fs`): `fs.readFile(path)` (ver `docs/stdlib/desktop.md`).
- Content-Type por extensión: `.html` → `text/html`, `.css` → `text/css`, `.js` → `application/javascript`, `.json` → `application/json`, `.png` → `image/png`, default `application/octet-stream`.
- 404 si el archivo no existe (`fs.exists`).
- **Nota**: `fs` es un host call del nodo — es la excepción permitida (file I/O no lo cubre WASI, que no está implementado). No es una violación de "host delgado": es el módulo desktop documentado.

## F6. Vistas con CMX + renderer CLS puro (`modules/view.clsx`)

**Las vistas usan CMX** — la sintaxis nativa de maquetación de CLS (JSX-like,
ver `docs/lenguaje/cmx.md`). NO se inventa un motor de templates (nada de
`{{ }}`, `@if`, `@for`, Blade).

CMX construye un árbol `CmxValue { tag, props, children }` (DOM virtual).
**No serializa a HTML por sí solo** (`cmx_to_string` produce debug abreviado),
así que el framework incluye un **renderer en CLS puro** que recorre el árbol:

```clx
# modules/view.clsx — renderer CMX -> HTML (CLS puro, 0 host nuevo)
function e(s: String) -> String {
    # escape HTML: & -> &amp;, < -> &lt;, > -> &gt;, " -> &quot;, ' -> &#39;
}

function render(cmx: any) -> String {
    # kind=1 (texto): devuelve e(texto)
    # tag + props: "<tag nombre=\"valor\" ...>"
    # children: concat render() de cada hijo (si un hijo es Array, expandir cada elemento)
    # cierre: "</tag>"
}
```

**Vista de la demo** — una función CLS que devuelve CMX:

```clx
# views.clsx — las vistas son funciones CLS (no archivos .blade)
function home_view(titulo: String, items: Array) -> any {
    return (
        <div class="container">
            <h1>{titulo}</h1>
            <ul>
                {items.map((x: String) -> <li>{x}</li>)}
            </ul>
        </div>
    );
}
```

**Handler**:
```clx
app.get("/", (req: Request) -> Response {
    return Response.html(render(home_view("Bienvenido", ["A", "B", "C"])));
});
```

**Cómo se cubren los gaps de CMX (v1)**:
| Necesidad | Patrón CMX |
|---|---|
| Loops | `{arr.map((x: T) -> <li>{x}</li>)}` — el renderer expande arrays de children |
| Condicionales | `{cond ? a : b}` como expresión, o `if` fuera del marcado construyendo el CmxValue por partes |
| Layouts/herencia | Composición de funciones: `layout_base(titulo, body)` que recibe el CmxValue del body |
| Componentes reutilizables | Llamar la función y pasar el resultado: `{home_view(...)}` (los tags en mayúscula NO se invocan — CMX guarda la referencia sin ejecutar) |
| Raw HTML (sin escape) | El renderer escapa por defecto; una función `raw(html)` marca el valor para que el renderer no lo escape (solo para contenido confiable) |

**Escape**: `e()` en el renderer (texto y props). Sin escape = riesgo de
inyección HTML — documentar que TODO lo interpolado se escapa salvo `raw()`.

## F7. Demo completa (`examples/http-framework/`)

`main.clsx`:
```clx
import "fs" as fs;      # solo para static
import "modules/router" as router;
# ... (los módulos del framework como imports, o un solo archivo si se prefiere)

function main(args: String[]) -> int {
    var app = App();
    app.use(logging_middleware);
    app.get("/", (req: Request) -> Response {
        return Response.html(render(layout_base("Home", home_view("Bienvenido", ["A", "B", "C"]))));
    });
    app.get("/api/users", (req: Request) -> Response {
        return Response.json({ok: true, users: ["ana", "beto", "carla"]});
    });
    app.get("/users/{id}", (req: Request) -> Response {
        return Response.json({id: req.params["id"]});
    });
    app.static("/public", "./public");
    print("Servidor en http://localhost:8080");
    app.serve(8080);
    return 0;
};
```

`views.clsx` (las vistas son funciones CLS que devuelven CMX — no archivos de
template):

```clx
# layout_base: composicion de layout (no hay @extends en CMX; se compone)
function layout_base(titulo: String, body: any) -> any {
    return (
        <html>
            <head><title>{titulo}</title></head>
            <body>
                {body}
            </body>
        </html>
    );
}

# home_view: la vista (CMX puro)
function home_view(titulo: String, items: Array) -> any {
    return (
        <div class="container">
            <h1>{titulo}</h1>
            <ul>
                {items.map((x: String) -> <li>{x}</li>)}
            </ul>
        </div>
    );
}
```

`README.md` de la demo: cómo correr (`clx run main.clsx`) y probar:
```bash
curl http://localhost:8080/              # HTML renderizado (layout + vista CMX)
curl http://localhost:8080/api/users     # {"ok":true,...}
curl http://localhost:8080/users/42      # {"id":"42"}
curl http://localhost:8080/no-existe     # 404
```

## Verificación del dev (criterio de aceptación)

1. `clx run examples/http-framework/main.clsx` arranca sin error.
2. Los 4 curls de arriba responden con lo esperado.
3. El servidor cierra limpiamente con Ctrl+C (o al menos no deja el puerto ocupado en el próximo run — manejar el error de `bind` en uso).
4. No se tocó ningún archivo de `cls-core/`, `cls-jit/`, `cls-internals/`, `nodos/clx/src/modules/`.
5. Suites de auditoría siguen verdes:
   ```powershell
   powershell -File examples/audit/test-features/jit-test/run-tests.ps1     # 28 PASS
   powershell -File examples/audit/test-features/jit-test/run-availible.ps1 # 24 PASS
   ```

## Limitaciones documentadas (v1)

| Limitación | Detalle |
|---|---|
| Sin concurrencia | Servidor bloqueante, un request a la vez. Concurrencia futura: threads vía `extension` a `pthread_create`, o `select`/`poll`. NO implementar en v1. |
| Chunked encoding | Solo `Content-Length`. `Transfer-Encoding: chunked` no soportado (documentar). |
| Keep-alive | Conexión se cierra tras cada request (Connection: close). |
| Vistas CMX | El renderer CLS puro cubre tags/props/children y expande arrays. Loops vía `{arr.map()}`, condicionales vía `{cond ? a : b}` o `if` fuera del marcado, layouts por composición de funciones. Sin escape de raw salvo `raw()` explícito. |
| Seguridad | Sin HTTPS/TLS. Body parseado como texto. Sin rate limiting. Documentar que es para desarrollo/educación, no producción. |

## Preguntas que el dev debe resolver (con sugerencia)

| Pregunta | Sugerencia |
|---|---|
| ¿Un archivo por módulo o todo en main? | Módulos separados en `modules/` + imports (más limpio, testable) |
| ¿Cómo se pasa `req.params`? | Campo `params: Record<String, String>` en Request, llenado por el router |
| ¿Ctor de Socket con `main`? | Sí (regla 4). El ctor recibe el fd |
| ¿Buffer de recv? | Alocar un string de capacidad fija (ej. 8192) por request; crecer si `Content-Length` lo pide |
| ¿Cómo distinguir en el renderer un child texto de un child CMX? | El child texto tiene kind=1 (tag es el texto plano); los elementos tienen tag + props. Ver `cmx.md` (Representación) y los tests `b8-cmx.clsx`/`10-cmx.clsx` |
