# Framework HTTP (minilaravel) — ejemplo

> **Estado**: ESPECIFICACIÓN lista. La implementación la hace un dev asignado,
> siguiendo `docs/desarrollo/minilaravel.md` (auto-contenida).
>
> Este directorio contiene el ejemplo completo del framework. Los módulos del
> framework (`modules/socket.clsx`, `modules/http.clsx`, `modules/router.clsx`,
> `modules/middleware.clsx`, `modules/static.clsx`, `modules/view.clsx`) y la
> demo (`main.clsx` + `views.clsx`) son los entregables del dev.

## Cómo correr (cuando esté implementado)

```bash
clx run main.clsx
```

## Cómo probar

```bash
curl http://localhost:8080/              # HTML renderizado (layout + vista CMX)
curl http://localhost:8080/api/users     # {"ok":true,"users":[...]}
curl http://localhost:8080/users/42      # {"id":"42"}
curl http://localhost:8080/no-existe     # 404
```

## Estructura esperada

```
examples/http-framework/
├── main.clsx            # demo: rutas /, /api/users, /users/{id}, static
├── modules/             # el framework (socket, http, router, middleware, static, view)
├── views.clsx           # vistas de la demo: funciones CLS que devuelven CMX
├── public/              # archivos estaticos de la demo
└── README.md            # este archivo
```

## Nota sobre las vistas

Las vistas usan **CMX** (la sintaxis nativa de maquetación de CLS), no un motor
de templates. `modules/view.clsx` implementa el renderer CMX → HTML en CLS puro
(recorre `.tag/.props/.children`). Ver F6 en la especificación.

## Referencias para el dev

- Especificación: `docs/desarrollo/minilaravel.md`
- Patrón de sockets por SO: `docs/lenguaje/extension-when.md`
- FFI `extension`: `docs/lenguaje/extension.md`
- CMX (sintaxis de vistas): `docs/lenguaje/cmx.md`
- Estado del runtime (WASI/async): `docs/lenguaje/estado-runtime.md`
