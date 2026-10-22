# Instalación

CLS se compila desde fuente con **Cargo**. El proyecto es un workspace Rust
(edición 2021) con siete crates: `cls-core`, `cls-runtime`, `cls-jit`,
`cls-internals` y los nodos `nodos/clx`, `nodos/clxb`, `nodos/clxr`.

## Requisitos

| Requisito | Detalle |
|---|---|
| Rust | Edición 2021 (`rustup` recomendado) |
| Cargo | Incluido con Rust |
| `wasm32-unknown-unknown` | `rustup target add wasm32-unknown-unknown` (lo usa `cls-internals/build.rs`) |
| `gcc` (MinGW-w64 / build-essential) | Solo para compilar los shims de `modules/` (Tombstone, sockets, SQLite, FFI) |

Para desarrollo, CLS se construye localmente. Las releases publicadas incluyen
portables con los módulos estándar; el `cargo build` normal no copia módulos.

## Clonar el repositorio

```ps
git clone https://github.com/frask/cls
cd cls
git submodule update --init --recursive
```

> Los proyectos de `cls-repositories/` son **submódulos** independientes
> (`tombstone`, `create-tombstone-app`). Sin `--recursive` esas carpetas quedan
> vacías.

> El remoto `origin` del repositorio actual apunta a
> `https://github.com/elfrask/cls.git`; el metadata del workspace
> (`Cargo.toml`) declara `https://github.com/frask/cls` como homepage y
> repository.

## Compilar

Desde la raíz del workspace:

```ps
cargo build -p clx     # CLI de desarrollo (clx)
cargo build -p clxr    # runtime ligero (clxr)
cargo build -p clxb    # bindings C (clsb.dll)
cargo build            # todo el workspace
```

El perfil de desarrollo compila las dependencias pesadas
(wasmtime + Cranelift) con `opt-level = 2`
(`[profile.dev.package."*"]` en el `Cargo.toml` raíz), de modo que el JIT
compila WASM -> nativo mucho más rápido incluso en build debug.

### Módulos y shims nativos

Los módulos de `/modules` que usan FFI necesitan su shim compilado antes de
ejecutar (ver `stdlib/modulos.md`):

```powershell
powershell -File modules\io\build.ps1
powershell -File modules\net\build.ps1
powershell -File modules\socket\build.ps1
powershell -File modules\sqlite\build.ps1
powershell -File modules\csprng\build.ps1
bash modules/argon2/build.sh
```

En Linux/macOS: `bash scripts/build-standard-modules.sh`. Los shims se resuelven
contra el `cls.json` de cada módulo, no contra el CWD.

### Binarios generados

| Binario | Crate | Propósito |
|---|---|---|
| `target/debug/clx.exe` | `clx` | CLI de desarrollo |
| `target/debug/clxr.exe` | `clxr` | Runtime ligero |
| `target/debug/clsb.dll` | `clxb` | Bindings C (nombre de lib `clsb`) |

## Scripts

Los scripts en `scripts/` delegan al binario de `target/debug`:

| Script | Acción |
|---|---|
| `clx.cmd` / `clx.sh` | Ejecuta `target/debug/clx.exe %*` |
| `clxr.cmd` / `clxr.sh` | Ejecuta `target/debug/clxr.exe %*` |
| `clx-build.cmd` / `clx-build.sh` | `cargo build --bin clx` |
| `clxr-build.cmd` / `clxr-build.sh` | `cargo build --bin clxr` |

En **Windows** se recomienda usar los `.cmd` (delegan al `.exe` de debug):

```ps
scripts\clx.cmd run main.clsx
scripts\clx-build.cmd
```

En Linux/macOS:

```sh
./scripts/clx.sh run main.clsx
```

## Verificación

```ps
clx -v
```

Imprime dos líneas: `clx 2.0.0` y `CLS Language Compiler & Runtime`.

```ps
clx -h
```

Muestra la ayuda completa con el encabezado `clx 2.0.0 - CLS Toolchain`.

Para probar una ejecución real:

```ps
clx run examples/test/hello/src/main.clsx
```

Debe imprimir `Hello from CLS!` y salir con código 0.

## Releases portables

Los artefactos de release incluyen:

```text
portable/
  clx[.exe]
  clxr[.exe]
  modules/
```

`clx` busca `modules/` junto al binario (`{dir_clx}/modules`), por lo que el
portable funciona sin copiar módulos a `~/.cls/modules`. En el repo, esa misma
búsqueda también cubre `{dir_clx}/../modules`.

La copia de `modules/` ocurre **solo en `release-pack.sh`**, cuando se generan
los binarios de release; no en builds de desarrollo.
