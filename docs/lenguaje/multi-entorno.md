# Multi-entorno: la directiva `when`

La directiva `when` selecciona código por **entorno objetivo** (SO,
arquitectura, ABI). Es una directiva compile-time/runtime: en el JIT la rama
que matchea el target se emite y las demás se descartan; en el walker las
ramas inactivas crean funciones "fantasma".

## Sintaxis

```clx
when os: windows {
    extension "msvcrt.dll" as C { ... };
};

when os: linux { ... };
when arch: arm64 { ... };
```

- Prefijos: `os:`, `arch:`, `abi:`, `platform:`, `target:` (tripla
  `arch-os-abi`).
- Combinaciones con `and` / `or` / `not` y paréntesis:

```clx
when os: none and arch: arm64 {
    # implementación nativa
};
```

- Nombres simples: un SO conocido (`windows`, `linux`, `macos`, `none`,
  `bare-metal`, `freebsd`) o una arquitectura conocida (`x86_64`, `arm64`,
  `aarch64` -> `arm64`, `arm`, `riscv32`, `riscv64`, `avr`) se interpretan
  según el prefijo.
- Una tripla con guiones (`x86_64-windows-msvc`) se interpreta como `target:`.
- `default { ... };` es la rama que siempre matchea.

## Simulación de entorno

`clx run --target <tripla>` (o `-t`) simula el entorno para la directiva:

```
clx run app.clsx --target x86_64-windows-msvc
```

## Comportamiento por intérprete

| Intérprete | Ramas inactivas |
|---|---|
| **Walker** | Las declaraciones se registran como "funciones fantasma": si se invocan sin que la rama activa definiera el símbolo, lanzan `No hay implementación de 'X' para el entorno actual (arch-os-abi)` |
| **JIT** | Se descartan en compile-time: solo se emite la rama que matchea `Target` (`WasmBackend`, `Statement::When`) |

El target por defecto es el del host (`Target::host()`); el typeck y el
backend lo reciben para evaluar las condiciones.