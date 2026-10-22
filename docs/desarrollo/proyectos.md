# Proyectos reales (`/cls-repositories`)

`/cls-repositories` es el espacio para **proyectos reales** construidos con CLS
2.0-dev2. No son pruebas de `examples/` ni módulos compartidos de `/modules`:
son aplicaciones y frameworks que se usan de verdad.

## Proyectos

| Proyecto | Descripción | Repositorio |
|---|---|---|
| `tombstone` | Framework web (App, router, CMX, ORM, auth, consola). | Submódulo `https://github.com/elfrask/tombstone` |
| `create-tombstone-app` | Plantilla/scaffold de una app Tombstone. | Submódulo `https://github.com/elfrask/create-tombstone-app` |

Cada subcarpeta es un **repositorio independiente** con su propio `.git`,
`cls.json`, README, tests y runner. El repo CLS solo guarda el **puntero al
commit** del submódulo.

```bash
git submodule update --init --recursive
```

## Reglas

1. **Autocontención.** Todo se construye en CLS puro + FFI. Prohibido crear
   `host_calls` o intrínsecos nuevos para tapar carencias
   (`examples/patrones.md` §2.4 y §5.4).
2. **Reutilizar, no copiar.** Los módulos compartidos viven en `/modules` y se
   consumen con `pathModules` o `--add-path-module`
   (ver `stdlib/modulos.md`).
3. **Lenguaje completo.** Usar clases/herencia, `match`, enums con payload,
   `for each`, `try/catch`, CMX, genéricos, etc.; no escribir un subconjunto
   cómodo (`examples/patrones.md` §5.6).
4. **Proyecto standalone.** Debe poder clonarse y correr solo: `cls.json`,
   `README.md`, `src/`, `tests/` y un runner (`run.ps1`/`run.sh`).
5. **Reproducible y verificable.** Cada proyecto define su oráculo externo
   (golden files, C vía FFI, CLI estándar) y sus tests fallan de forma clara.
6. **Sin tocar el host.** Si falta una capacidad, se resuelve con FFI + CLS; si
   no se puede, es un bug del lenguaje y se reporta.

## Estructura

```text
cls-repositories/<proyecto>/
├── .git/
├── cls.json
├── README.md
├── src/
│   └── main.clsx
├── tests/
├── run.ps1            # o run.sh
└── .gitignore
```

`cls.json` mínimo:

```json
{
  "name": "mi-proyecto",
  "version": "0.1.0",
  "entry": "src/main.clsx",
  "testEntry": ["tests/**/*.clsx"],
  "pathModules": ["../../modules"]
}
```

- `pathModules` es relativo a la **raíz del proyecto**; desde
  `cls-repositories/<proyecto>/` apunta a `/modules` del repo CLS con
  `../../modules`.
- Los errores de runtime se muestran con trazo completo; el typecheck, de un
  solo nivel.

## Ejecutar desde el repo CLS

```powershell
target\debug\clx.exe check cls-repositories\<proyecto>\src\main.clsx
target\debug\clx.exe run   cls-repositories\<proyecto>\src\main.clsx
target\debug\clx.exe test  cls-repositories\<proyecto>
```

## Agregar un proyecto

```bash
cd cls-repositories
mkdir <proyecto>
cd <proyecto>
git init
# crear cls.json, README.md, src/, tests/ y runner
```

Cada proyecto se versiona en **su propio repositorio**; el repo padre CLS no
debe absorber su contenido. Para Tombstone, ver `desarrollo/minilaravel.md`.
