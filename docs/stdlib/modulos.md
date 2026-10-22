# Módulos compartidos (`/modules`)

`/modules` es la **stdlib autocontenida** del repo CLS: módulos fuente
reutilizables por los proyectos de `examples/` y `cls-repositories/`. Se
construyen con **CLS puro + FFI** y son la base del framework Tombstone.

> No confundir con `./modules` de un proyecto (módulos propios) ni con
> `~/.cls/modules` (paquetes instalados por `clx install`).

## Reglas

1. **CLS puro + FFI.** Nada de `host_calls` ni intrínsecos nuevos. Si falta una
   capacidad, se resuelve con `extension`.
2. **Sin dependencias externas** fuera de CLS + FFI.
3. **Un módulo = una carpeta** en `modules/<nombre>/` con su `cls.json`.
4. **Reutilizar, no copiar.** Se referencian con `pathModules` o
   `--add-path-module` (ver `docs/lenguaje/modulos.md`).
5. Si un módulo reemplaza un `host_call` (`fs`, `http`, `json`, `strings`), se
   anota en la tabla de deuda de `examples/patrones.md` §2.7.

## Estructura

```text
modules/<nombre>/
├── cls.json          # { "name": "...", "version": "...", "export": { "entry": "mod.clsx" } }
├── mod.clsx          # barrel: reexporta solo la API pública
├── <dominio>.clsx    # una responsabilidad por archivo
└── <mod>_ffi.clsx    # bindings FFI crudos (no públicos)
```

- Los imports internos se resuelven desde la **raíz del módulo**.
- Los helpers internos se exportan entre archivos pero **no** se reexportan en
  el barrel.
- Las reexportaciones de tipos conservan el nombre original (`x::Clase as Clase`);
  funciones y constantes admiten renombrado.
- **Stems únicos**: el typeck registra los módulos internos por el nombre del
  archivo. Dos `.clsx` con el mismo stem en módulos distintos del grafo
  colisionan; usá `io_ffi.clsx`, `net_ffi.clsx`, etc.

## Consumo desde un proyecto

```json
{
  "name": "mi-app",
  "version": "0.1.0",
  "entry": "src/main.clsx",
  "pathModules": ["../../modules"]
}
```

```bash
clx run src/main.clsx --add-path-module=../../modules
clx test --add-path-module=../../modules
```

`pathModules` es relativo a la raíz del proyecto; el flag es relativo al CWD y
es repetible. `clx test` aplica `pathModules` del `cls.json` del CWD y del
proyecto de cada test.

## Catálogo

| Módulo | Tipo | Notas |
|---|---|---|
| `io` | FFI | I/O de archivos y descriptores sobre `io_shim`; reemplaza `host_call fs`. |
| `bytes` | CLS puro | Primitivas de byte/buffer, hex, big-endian y `firstDiff`. |
| `prng` | CLS puro | LCG de 64 bits (`Prng`); no es criptográfico. |
| `check` | CLS puro | Aserciones y protocolo `CHECK k=v` para runners. |
| `config` | CLS puro | Introspección de solo lectura sobre el intrinsic `config()`. |
| `env` | CLS puro | Carga de `.env` (`Env.get/require/getInt/getBool`). Ver `stdlib/env.md`. |
| `orm` | CLS puro | ORM agnóstico al motor + contrato `Driver`, `Cell`, `ResultSet`. |
| `sqlite` | FFI | Driver SQLite sobre `sqlite_shim`; consume `orm`. |
| `net` | FFI | Sockets UDP/TCP sobre `net_shim` (handles enteros). Usado por `dns`. |
| `dns` | CLS puro | Codec DNS RFC 1035 y servidor autoritativo de ejemplo. |
| `socket` | FFI | Sockets TCP persistentes/bidireccionales sobre `socket_shim`. Base de `httpc`/`httpd`. |
| `httpc` | CLS puro | Cliente HTTP/1.1 sobre `socket` (redirects, cookies, keep-alive; solo `http://`). |
| `httpd` | CLS puro | Servidor HTTP/1.1 sobre `socket` (`Request`, `Response`, `serve`). Usado por Tombstone. |
| `argon2` | FFI | Hash de passwords Argon2id sobre `libargon2` (shim con la lib estática). |
| `csprng` | FFI | CSPRNG del sistema (`BCryptGenRandom` / `/dev/urandom`). |

El detalle de la API pública de cada módulo vive en su `mod.clsx` y en
[`modules/README.md`](https://github.com/elfrask/cls/blob/master/modules/README.md).

## Shims nativos

Los módulos FFI necesitan su shim compilado **antes** de ejecutar:

```powershell
powershell -File modules\io\build.ps1
powershell -File modules\net\build.ps1
powershell -File modules\socket\build.ps1
powershell -File modules\sqlite\build.ps1
powershell -File modules\csprng\build.ps1
bash modules/argon2/build.sh
```

En Linux/macOS, la CI usa `scripts/build-standard-modules.sh`. Las rutas de los
shims se resuelven contra el `cls.json` de cada módulo (`pathModulesNatives`),
no contra el CWD. `clx build` copia las nativas usadas a
`dist/native-libs/<paquete>@<version>/`.

## Empaquetado

`modules/` se copia junto a `clx`/`clxr` **solo al empaquetar una release**
(`scripts/release-pack.sh`); un `cargo build` normal no la incluye. El portable
funciona sin instalación porque el resolver busca `{dir_clx}/modules` (ver
`guia/instalacion.md`).
