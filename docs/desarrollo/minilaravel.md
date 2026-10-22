# Tombstone — framework HTTP en CLS puro

Tombstone es el framework web de CLS 2.0: router, middleware, HTTP/1.1,
archivos estáticos, CORS, cookies, JSON, vistas CMX, plantillas, validación,
controllers, ORM, migraciones, auth y consola. Se construye con **CLS puro +
FFI**: no agrega `host_calls` ni intrínsecos al lenguaje.

> Este documento reemplaza la especificación histórica de “minilaravel”. La
> implementación de referencia vive en el submódulo
> [`cls-repositories/tombstone`](https://github.com/elfrask/tombstone), y la
> plantilla para crear una app en
> [`cls-repositories/create-tombstone-app`](https://github.com/elfrask/create-tombstone-app).

## Ubicación

| Pieza | Ruta |
|---|---|
| Motor del framework | `cls-repositories/tombstone/modules/tombstone/` |
| Iconos CMX | `cls-repositories/tombstone/modules/tombstone-icons/` |
| App demo / template | `cls-repositories/tombstone/src/` + `public/` |
| Scaffold de apps | `cls-repositories/create-tombstone-app/` |
| Dependencias compartidas | `/modules/socket`, `/modules/httpd`, `/modules/orm`, `/modules/sqlite`, `/modules/check`, `/modules/csprng`, `/modules/argon2` |

> El motor **no** está en `/modules/tombstone`: vive dentro del submódulo. Para
> consumirlo desde otro proyecto hay que agregar las dos rutas a `pathModules`
> (ver “Consumo como módulo”).

## Arquitectura

```text
App (router + middleware + static + CORS)
  └── Request / Response (validación de forms, multipart y JSON)
        └── Controller (json/ok/created/invalid/redirect)
              └── DTO → Action → Repository
                    └── Model / Query / ModelRow
                          └── modules/orm → modules/sqlite
```

- **Transporte**: `modules/socket` (TCP persistente, FFI) + `modules/httpd`
  (parser y loop HTTP/1.1 en CLS puro).
- **Vistas**: CMX + `cls-repositories/tombstone/modules/tombstone/view.clsx`
  (CMX → HTML con escape, `raw`, void elements, componentes y `renderEach`).
- **Plantillas**: `Template` (title, metas, styles, scripts, favicon, body class,
  `derive`).
- **Seguridad**: Argon2id (`modules/argon2`), CSPRNG (`modules/csprng`),
  sesiones SQLite, CSRF, `AuthGuard`, policies y rate limit.
- **Aplicación**: `EventBus` + model events, `Logger`, `Action`/`Service`,
  providers y DTOs.

## Estructura del proyecto

```text
tombstone/
├── cls.json
├── modules/
│   ├── tombstone/          # motor (App, router, controller, model, ...)
│   │   ├── cls.json        # export.entry = mod.clsx
│   │   ├── mod.clsx        # barrel público
│   │   ├── types/          # interfaces de props CMX (HTML + UI)
│   │   ├── bin/            # launcher global
│   │   └── setup.clsx
│   └── tombstone-icons/
├── src/
│   ├── main.clsx           # rutas + demo
│   ├── components.clsx     # componentes CMX
│   ├── controllers.clsx
│   ├── config.clsx
│   ├── dtos.clsx
│   ├── repositories.clsx
│   ├── actions.clsx
│   ├── services.clsx       # ApplicationServices + providers
│   ├── resources.clsx
│   ├── scopes.clsx
│   ├── migrations.clsx
│   └── views.clsx
├── public/                 # css / js / favicon
└── tests/
    ├── unit/
    ├── db/
    └── integration/
```

`create-tombstone-app` genera la misma base (`app/`, `database/`, `config/`,
`public/`, `tests/`, `.env.example`, `cls.json`).

## Crear y ejecutar una app

Instalación del launcher global (una vez):

```powershell
powershell -File cls-repositories\tombstone\modules\tombstone\scripts\install-global.ps1
```

```bash
bash cls-repositories/tombstone/modules/tombstone/scripts/install-global.sh
```

Agregá `~/.cls/bin` al `PATH`. También podés usar `clx start`, que construye el
`PATH` con los `bin/` de los módulos resueltos:

```bash
clx start tombstone migrate:status
```

Crear una app en una carpeta vacía:

```powershell
tombstone create app
```

Correr el demo desde la raíz del repo CLS:

```powershell
target\debug\clx.exe run cls-repositories\tombstone\src\main.clsx -- 8080
```

Con el launcher (desde la raíz del proyecto):

```powershell
tombstone serve 8080
tombstone test
tombstone check
tombstone build
```

Build de la app (desde su directorio):

```powershell
target\debug\clx.exe build
```

Genera `dist/app.clsapp` y `dist/native-libs/<paquete>@<version>/`. El
`.clsapp` es un zip de **código fuente**; las nativas se copian aparte.

## API principal

| Área | API |
|---|---|
| `App` | `get/post/put/patch/delete_route/options`, `controller`, `group`, `route`, `use`, `serve_static`, `cors`, `handle(raw)`, `dispatch(Request)`, `serve(host, port)` |
| `Router` | `add`, `add_named`, `url`, `controller`, verbos, `{param}`, `{id?}`, named routes, 404/405 |
| `Middleware` | `(Request) -> Response`; `status == 0` continúa la cadena |
| `Request` | `http_method`, `path`, `query`, `query_param`, `headers`, `header`, `body`, `params`, `param`, `form`, `files`, `input`, `all`, `validate`, `validate_schema`, `file`, `has_file` |
| `Response` | `text`, `html`, `json_raw`, `json_response`, `redirect`, `status_only`, `not_found`, `header`, `set_cookie`, `add_cookie`, `serialize` |
| `Controller` | `input/all/param/file`, `validate`, `validate_schema`, `json/ok/created/accepted/no_content/invalid/error/redirect` |
| `Template` | `set_*`, `add_style`, `add_script`, `add_meta`, `add_meta_prop`, `derive`, `render(page)` |
| `view` | `e`, `raw`, `render`, `renderChildren`, `attrs`, `isVoid`, `renderEach`, `layout`, `ul_items` |
| `Model` | CRUD, `where`, `paginate`, agregados, `first_or_create`, `update_or_create`, relaciones, soft deletes |
| `Query` | `where_eq/like/in/null/not_null/between`, `order_by`, `limit/offset`, agregados, `update/delete` |
| `DbSchema` | `blueprint`, `create`, `alter`, `drop`, `has_table`, `has_column` |
| `MigrationRunner` | `add`, `run`, `rollback`, `reset`, `status` |
| `Factory` | `make`, `create`, `create_many` |
| Auth | `hash_password`/`verify_password`, `AuthGuard`, `Csrf`/`CsrfMiddleware`, `Session`/`DbSessionStore`, `Gate`, `RateLimiter` |
| Utilidades | `EventBus`/`Event`/`Listener`, `Logger`, `Action`/`Service`, `Cache`/`FileCache`, `TestClient`/`TestResponse`, `HttpStatus` |

Ejemplo mínimo:

```clx
from "tombstone" import App, Request, Response, Template, render;

function main(args: String[]) -> int {
    var app = App();
    app.get("/", (req: Request) -> Response {
        var t = Template();
        t.set_title("Inicio");
        return Response.html(render(t.render(<h1>{"Hola"}</h1>)));
    });
    return app.serve("127.0.0.1", 8080);
};
```

## Tests

Con el launcher, desde la raíz del proyecto:

```powershell
tombstone test:all
tombstone test:unit
tombstone test:db
tombstone test:http
```

O los runners directamente, desde la raíz del repo CLS:

```powershell
powershell -File cls-repositories\tombstone\tests\unit\run.ps1
powershell -File cls-repositories\tombstone\tests\db\run.ps1
powershell -File cls-repositories\tombstone\tests\integration\http_smoke.ps1
```

Estado actual: **13 tests unitarios**, **75 checks DB** y **71 checks HTTP**
(0 FAIL). La integración levanta el servidor real y usa `curl`; el `TestClient`
permite requests in-process sin sockets.

Prerrequisitos: shims de `io`, `socket`, `sqlite`, `csprng` y `argon2`
compilados (ver `stdlib/modulos.md`).

## Consola, seeds y generadores

```powershell
tombstone migrate
tombstone migrate:rollback
tombstone migrate:status
tombstone db:seed
tombstone make:controller Home
tombstone make:model Note
tombstone make:migration create_notes
tombstone make:factory Note
tombstone make:action CreateNote
tombstone make:resource Note
```

- El launcher usa `CLX` (ruta de `clx`) y `TOMBSTONE_CONSOLE` (default
  `src/console.clsx`).
- Los seeders son idempotentes; con `TOMBSTONE_DB=archivo.db` persisten.
- El setup del launcher vive en `modules/tombstone/bin` + `setup.clsx`;
  `clx install` lo ejecuta una vez por versión y copia los bins a `~/.cls/bin`.

## Configuración de seguridad

| Variable | Efecto |
|---|---|
| `TOMBSTONE_SECURE_COOKIE=1` | Agrega `Secure` a la cookie de sesión (requiere HTTPS). |
| `TOMBSTONE_DB=archivo.db` | Sesiones persistentes en SQLite (default `:memory:`). |
| `TOMBSTONE_ENV=production` | Desactiva `debug`; no cambia el KDF. |

CSRF: el login devuelve `X-CSRF-Token`; también `GET /api/csrf`. En métodos
mutantes con cookie se acepta `X-CSRF-Token` o `_csrf`. Rate limit global:
300 req/min por IP, además de 5/min en `/api/login`.

## Consumo como módulo

En el `cls.json` del proyecto consumidor:

```json
{
  "pathModules": [
    "<ruta-al-repo-cls>/modules",
    "<ruta-al-repo-cls>/cls-repositories/tombstone/modules"
  ]
}
```

```clx
from "tombstone" import App, Request, Response, Template, render;
from "tombstone" import AnchorProps, ButtonProps, DivProps;
from "tombstone-icons" import icon;
```

El consumidor también necesita `modules/socket` y `modules/httpd`; si usa Model,
`modules/orm` y `modules/sqlite`. `static` usa el módulo `fs` del nodo desktop.

## Limitaciones conocidas

- HTTP/1.1 sin `Transfer-Encoding: chunked` ni keep-alive: una request por
  conexión (`Connection: close`).
- Sin concurrencia: el loop es bloqueante (no hay `async` ejecutable).
- `headers` es `Record<String, String>`; los `Set-Cookie` múltiples usan
  `Response.add_cookie`.
- Router sin constraints (`where`); sí tiene `{id?}`, named routes y
  controllers como clases.
- CMX: un child que es `Array` no se expande solo; usar `renderEach` + `raw`.
  Los componentes uppercase deben devolver `Cmx`.
- Spread de props: el source debe ser `Record<String, Value>`; convertí el
  interface (Shape) antes de `<Componente {...props} />`.
- JSON: parser permisivo; `json.stringify(array)` no es fiable, usar
  `json_string_array`.
- `static`/sockets/IO resuelven los shims contra el `cls.json` del módulo, no
  contra el CWD.

## Deudas del lenguaje encontradas

- **Identificador local `path`**: `path.miembro` se parsea como namespace del
  módulo interno `path` aunque `path` sea un parámetro/local. En el motor se usa
  `route_path`/`file_path`.
- **Flatten de exports hermanos**: si una clase exportada llama a otra función
  exportada del mismo módulo y el consumidor no la importa, el flatten puede
  omitirla. El barrel importa explícitamente los símbolos hermanos que usa.

## Historia

La especificación original “minilaravel” definía las fases F1–F7; la
implementación actual las cubre así:

| Fase | Implementación actual |
|---|---|
| F1 sockets TCP | `modules/socket` (FFI) |
| F2 parser HTTP | `modules/httpd` |
| F3 router | `tombstone/modules/tombstone/router.clsx` |
| F4 middleware | `tombstone/modules/tombstone/middleware.clsx` |
| F5 static | `tombstone/modules/tombstone/static.clsx` |
| F6 vistas CMX | `tombstone/modules/tombstone/view.clsx` |
| F7 demo | `src/main.clsx` + `public/` |

Ver también `desarrollo/proyectos.md` y `stdlib/modulos.md`.
