# Framework HTTP (minilaravel) — ejemplo

> **Estado**: ESPECIFICACIÓN lista. La implementación la hace un dev asignado,
> siguiendo `docs/desarrollo/minilaravel.md` (auto-contenida).
>
> Este directorio contiene el ejemplo completo del framework. Los módulos del
> framework (`modules/socket.clsx`, `modules/http.clsx`, `modules/router.clsx`,
> `modules/middleware.clsx`, `modules/static.clsx`, `modules/view.clsx`) y la
> demo (`main.clsx` + `views/`) son los entregables del dev.

## Cómo correr (cuando esté implementado)

```bash
clx run main.clsx
```

## Cómo probar

```bash
curl http://localhost:8080/              # view con layout
curl http://localhost:8080/api/users     # {"ok":true,"users":[...]}
curl http://localhost:8080/users/42      # {"id":"42"}
curl http://localhost:8080/no-existe     # 404
```

## Estructura esperada

```
examples/http-framework/
├── main.clsx            # demo: rutas /, /api/users, /users/{id}, static
├── modules/             # el framework (socket, http, router, middleware, static, view)
├── views/               # templates (base.blade, home.blade)
├── public/              # archivos estaticos de la demo
└── README.md            # este archivo
```

## Referencias para el dev

- Especificación: `docs/desarrollo/minilaravel.md`
- Patrón de sockets por SO: `docs/lenguaje/extension-when.md`
- FFI `extension`: `docs/lenguaje/extension.md`
- Estado del runtime (WASI/async): `docs/lenguaje/estado-runtime.md`
