# Instalación

CLS se compila desde fuente con **Cargo**. El proyecto es un workspace Rust
(edición 2021) con seis crates: `cls-core`, `cls-runtime`, `cls-jit` y los
nodos `nodos/clx`, `nodos/clxb`, `nodos/clxr`.

## Requisitos

| Requisito | Detalle |
|---|---|
| Rust | Edición 2021 (`rustup` recomendado) |
| Cargo | Incluido con Rust |

No hay binarios precompilados: se construye localmente.

## Clonar el repositorio

```ps
git clone https://github.com/frask/cls
cd cls
```

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
compila WASM → nativo mucho más rápido incluso en build debug.

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

Muestra la ayuda completa con el encabezado `clx 2.0.0 — CLS Toolchain`.

Para probar una ejecución real:

```ps
clx run examples/hello/src/main.clsx
```

Debe imprimir `Hello from CLS!` y salir con código 0.
