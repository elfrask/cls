# Directiva `when` — implementaciones multi-entorno

La directiva `when` declara **implementaciones alternativas por entorno**
(sistema operativo, arquitectura, ABI, plataforma/HAL). Aplica a **cualquier
tipo de declaración** (extension, function, var, class, enum, module).

> Esta es la feature oficial. Detalles de implementación (entidad multi-entorno,
> fantasmas, portabilidad) en `agent-context/MULTI_TARGET.md`.

## Sintaxis

```clx
when os: linux {
    function saludo() -> String { return "hola linux"; }
}
when os: windows {
    function saludo() -> String { return "hola windows"; }
}
when os: macos { ... }
default { ... }              # rama que siempre matchea
```

Combinaciones de condiciones:

```clx
when os: none and arch: riscv32 { ... }     # bare-metal RISC-V
when linux or macos { ... }                 # and / or / not / !
when platform: esp32 { ... }                # por placa/HAL
when target: "riscv32-none-elf" { ... }     # tripla de target
```

## Selectores de condición

| Selector | Valores |
|----------|---------|
| `os:` | `windows`, `linux`, `macos`, `none` (bare-metal), `freebsd`, `bare-metal` |
| `arch:` | `cls-arch` (arquitectura nativa CLS), `x86_64`, `arm64`, `aarch64`, `arm`, `riscv32`, `riscv64`, `avr` |
| `abi:` | `gnu`, `msvc`, `eabi`, `elf` |
| `platform:` | `pc`, `esp32`, `stm32f4`, `rp2040`, `none`, ... |
| `target:` | tripla `arch-vendor-os-abi` (ej. `riscv32-none-elf`) |

También se aceptan nombres simples: `when windows` = `os: windows`,
`when riscv32` = `arch: riscv32`.

> La arquitectura nativa de CLS es **`cls-arch`** (el target "presente" por
> defecto tanto en el intérprete como en los builds). `wasm` queda reservado.

## Entidad multi-entorno

Declaraciones con el **mismo nombre** en varias ramas `when` forman una
**entidad multi-entorno**:

- El símbolo **existe siempre** (contrato); su implementación se selecciona por
  el entorno actual.
- Si el entorno actual **no tiene implementación** para el símbolo, llamarlo da
  error claro: *"No hay implementación de 'x' para el entorno actual (...)"*.
- Los **exports/imports no cambian** para el consumidor: `m.f()` es portable.

```clx
# modulo.clsx
when linux   { export function icono() -> int { ... } }
when windows { export function icono() -> int { ... } }

# consumidor.clsx — portable
import "modulo" as m;
print(m.icono());   # usa la implementación del entorno actual
```

## Ejecución

```bash
clx run app.clsx                      # usa el target del host
clx run --target linux app.clsx       # simular un entorno
clx run --target riscv32-none-elf app.clsx
clx build --target <tripla> ...       # AOT embebido (futuro)
```

- **Intérprete/JIT**: ejecuta la rama que matchea el target actual (host o
  simulado).
- **Binario portable**: el `.clbin` quema todas las ramas y selecciona en runtime.
- **AOT embebido**: el target se fija en build; solo la rama aplicable se compila.

## Uso típico con `extension`

```clx
when os: windows {
    extension "user32" as C { function MessageBoxA(h: CPtr, t: CString, c: CString, u: CUInt) -> CInt; }
}
when os: linux {
    extension "libX11" as C { function XOpenDisplay(d: CString) -> CPtr; }
}
```

## Ver también

- `extension.md` — FFI a librerías nativas.
- `modulos.md` — `export`/`import` de símbolos.
- `agent-context/MULTI_TARGET.md` — plan de implementación y entidad `Target`.
