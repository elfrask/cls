# `/cls-repositories` — Proyectos reales de CLS 2.0-dev2

Espacio local para **proyectos reales** construidos con CLS 2.0-dev2. No son
pruebas (`examples/`) ni módulos compartidos (`modules/`): son aplicaciones y
frameworks que se usan de verdad.

Cada subcarpeta es un **repositorio independiente** (con su propio `.git`,
`cls.json`, README, tests y runner). Este archivo es el índice local.

## Proyectos

| Proyecto | Descripción | Estado |
|----------|-------------|--------|
| `framework/` | Framework web en CLS puro sobre FFI (sockets TCP + HTTP + router + vistas CMX). | En preparación |

> El primer proyecto es el **framework** de dev-2. Especificación:
> `docs/desarrollo/minilaravel.md` y
> `agent-context/dev2/07-FRAMEWORK-HTTP-DEFERRED.md`.

## Reglas

1. **Autocontención.** Todo se construye en CLS puro + FFI a C. Prohibido crear
   `host_calls` o intrínsecos nuevos para tapar carencias
   (`examples/patrones.md` §2.4, §5.4).
2. **Reutilizar, no copiar.** Los módulos compartidos viven en `/modules` y se
   consumen con `pathModules` o `--add-path-module` (`modules/README.md`).
3. **Lenguaje completo.** Usar las features disponibles (clases/herencia,
   `match`, enums con payload, `for each`, `try/catch`, CMX, genéricos, etc.);
   no escribir un subconjunto cómodo (`examples/patrones.md` §5.6).
4. **Proyecto standalone.** Debe poder clonarse y correr solo:
   `cls.json`, `README.md`, `src/`, `tests/` y un runner (`run.ps1`/`run.sh`).
5. **Reproducible y verificable.** Cada proyecto define su oráculo externo
   (golden files, C vía FFI, CLI estándar) y sus tests fallan de forma clara.
6. **Sin tocar el host.** Si falta una capacidad, se resuelve con FFI + CLS; si
   no se puede, es un bug del lenguaje y se reporta.

## Estructura de un proyecto

```
cls-repositories/<proyecto>/
├── .git/
├── cls.json
├── README.md
├── src/
│   └── main.clsx
├── tests/
├── run.ps1            (o run.sh)
└── .gitignore
```

`cls.json` mínimo:

```json
{
  "name": "framework",
  "version": "0.1.0",
  "entry": "src/main.clsx",
  "testEntry": ["tests/**/*.clsx"],
  "pathModules": ["../../modules"]
}
```

- `pathModules` es relativo a la **raíz del proyecto**; desde
  `cls-repositories/<proyecto>/` apunta a `/modules` del repo CLS con
  `../../modules`.
- `clx run` usa el **JIT** por defecto. `--ast-walker` está deprecado.
- Los errores de runtime se muestran con trazo completo (no mensajes sueltos).

## Cómo agregar un proyecto

```bash
cd cls-repositories
mkdir <proyecto>
cd <proyecto>
git init
# crear cls.json, README.md, src/, tests/ y runner
```

Luego, desde la raíz del repo CLS:

```bash
target/debug/clx.exe check cls-repositories/<proyecto>/src/main.clsx
target/debug/clx.exe run   cls-repositories/<proyecto>/src/main.clsx
target/debug/clx.exe test  cls-repositories/<proyecto>
```

Cada proyecto se versiona en **su propio repositorio**; el repo padre CLS no
debe absorber su contenido (evitar `git add` de subrepos anidados).

## Referencias

- `examples/patrones.md` — filosofía de autocontención y reglas de pruebas.
- `examples/PLAN-PRUEBAS.md` — proyectos de prueba y oráculos.
- `modules/README.md` — módulos compartidos y cómo consumirlos.
- `docs/desarrollo/minilaravel.md` — especificación del framework.
- `docs/lenguaje/` — referencia del lenguaje y del runtime.
