# Prueba Brainfuck — diferencial CLS puro vs oráculo C

Prueba de la fase final de CLS 2.0 (proyecto 3 de `examples/PLAN-PRUEBAS.md`,
mejor ROI). Ejecuta los mismos programas `.bf` con:

1. un **intérprete de Brainfuck escrito en CLS puro** (`src/bf.clsx`), y
2. un **intérprete de referencia en C** (`oracle/bf.c`, ~100 LOC) expuesto por
   **FFI** (oráculo externo independiente del lenguaje).

y compara la salida **byte a byte**, además de medir tiempos. Millones de
iteraciones (programa `bench`) exponen bugs de codegen del JIT.

Todo es **CLS puro + FFI a C**: no se usa `fs`/`http`/`json`/`strings` ni
`host_calls` nuevos, y **no se exponen punteros** al código CLS (el oráculo
devuelve `int[]`, no `CPtr`).

La I/O de archivos y las utilidades de bytes se consumen de **`/modules`**
(`examples/patrones.md` §11.5): `modules/io` (`readText`/`readBytes`, FFI) y
`modules/bytes` (`ord`, `firstDiff`, `hexEncode`). La prueba **no copia** ese
código; el único FFI propio es el del oráculo BF.

## Estructura

```
examples/brainfuck/
├── cls.json                 # { name, version, entry, pathModules }
├── src/main.clsx            # runner: lee .bf, corre CLS, corre C, compara, mide
├── src/bf.clsx              # intérprete BF en CLS puro + FFI al oráculo (bf_run)
├── oracle/bf.c              # intérprete BF de referencia en C (+ CLI de .out)
├── oracle/bf.dll            # compilado (gcc -shared)
├── oracle/bf_oracle.exe     # CLI del oráculo (regenera los .out, mide en C)
├── programs/*.bf            # programas de prueba
├── programs/*.out           # salida esperada (generada por el oráculo C)
├── programs/cat.in          # entrada para el programa `cat` (`, `)
├── findings/                # repros mínimos de las regresiones halladas
└── run.ps1                  # orquestador
```

Consume de `/modules`: `io` (I/O de archivos), `bytes` (`ord`, `firstDiff`,
`hexEncode`). Los `build.ps1` de `io` y `bytes` los llama `run.ps1`.

## Cómo correr

Desde la **raíz del repo** (CWD = raíz; el `.dll` del oráculo se resuelve
relativo al CWD):

```powershell
target\debug\clx.exe run examples\brainfuck\src\main.clsx --add-path-module=modules
```

O todo el ciclo (compila el `.dll` si falta, regenera los `.out`, corre):

```powershell
powershell -File examples\brainfuck\run.ps1
```

El `cls.json` también declara `"pathModules": ["../../modules"]` (equivalente al
flag `--add-path-module` cuando se corre con CWD = `examples/brainfuck`).

## Compilar el oráculo C

Compilador usado: **gcc 15.2.0 (MinGW-w64 x64)**, en `C:\mingw64\bin\gcc.exe`.

```powershell
gcc -shared -O2 -o oracle\bf.dll        oracle\bf.c
gcc -O2 -DBF_CLI -o oracle\bf_oracle.exe oracle\bf.c
```

`bf_oracle.exe <prog.bf> [--in entrada] [--out salida]` ejecuta un programa y
escribe la salida cruda (a `--out` o a stdout). Con `BF_ORACLE_VERBOSE=1` añade
`[oracle] bytes=N steps=M` a stderr.

## Programas

| Programa   | Qué ejercita | Salida |
|------------|--------------|--------|
| `hello.bf` | base, bucles, output | `Hello World!\n` (13 B) |
| `wrap.bf`  | wrap de celda 0↔255 y de puntero | 5 B |
| `torture.bf` | bucles anidados + output con wrap de byte (incluye NUL) | 51000 B |
| `bench.bf` | loop caliente determinista (~8 060 601 pasos) | 1 B |
| `ptr.bf`   | wrap del puntero por debajo de 0 (celda 29999) | 4 B |
| `cat.bf`   | `,` (entrada) y `.` (eco) | `Hi!` (3 B) |

## Resultado (JIT/wasmtime, Windows x64)

```
=== Brainfuck: CLS puro vs oraculo C (FFI) ===
  [ OK ] hello     len cls=13 oracle=13 golden=13 | t_cls=1ms t_c=0ms
  [ OK ] wrap      len cls=5 oracle=5 golden=5 | t_cls=1ms t_c=0ms
  [ OK ] torture   len cls=51000 oracle=51000 golden=51000 | t_cls=3ms t_c=0ms
  [ OK ] bench     len cls=1 oracle=1 golden=1 | t_cls=50ms t_c=10ms
  [ OK ] ptr       len cls=4 oracle=4 golden=4 | t_cls=0ms t_c=0ms
  [ OK ] cat       len cls=3 oracle=3 golden=3 | t_cls=1ms t_c=0ms
RESULTADO: OK (todos los programas coinciden byte a byte)
```

- **No hay divergencia** CLS↔C↔dorado en ningún programa (incluidos 51000 bytes
  con bytes NUL y wrap de celda/puntero).
- `bench` (8.06 M pasos): CLS ≈ 50–160 ms vs C ≈ 10–28 ms → el CLS es ~5× más
  lento, pero **compila de verdad** (no interpreta). El tiempo varía con el
  caché JIT (primera corrida más lenta).

## Regresiones encontradas

Ver `findings/README.md`. Resumen:

| # | Feature | Síntoma | Repro |
|---|---------|---------|-------|
| 1 | **FFI `extension` a nivel superior** | panic del emisor (`no entry found for key`, `emit.rs:623`, exit 101) | `findings/01-...clsx` |
| 2 | **Bitwise `&` / `\|`** | WASM inválido (`expected i32, found i64`, exit 1) | `findings/02-...clsx` |
| 3 | **`finally`** | no se ejecuta en salida normal ni tras `catch` local | `findings/03-...clsx` |
| 4 | **Carga de librería FFI** | error no fatal: continúa y sale con 0 | `findings/04-...clsx` |

## Gaps de lenguaje (rodeados, no bloqueantes)

- **Sin `ord`/`chr`** (al construir la prueba): no había forma de obtener el
  código de byte de un `String` de 1 byte (necesario para implementar `,`). Se
  resolvió con **FFI extraído a `modules/bytes`** (`bytes::ord`).
  `s[i]` devuelve un `String` de 1 byte, no un código.
- **Sin bytes crudos C→CLS**: un retorno `CString` se trunca en el primer NUL
  (usa `strlen`). Para leer bytes crudos (p. ej. el `.out` con NUL) se usa
  **`modules/io`** (`io::readBytes(path) -> int[]`, `CArray`).
- **Sin allocador/memoria en CLS**: no hay intrínseco para reservar un buffer ni
  para convertir un `CPtr`/byte a `String`, así que la I/O de bytes necesita un
  helper C (FFI). `fopen`/`fgetc`/`fclose` con `CPtr` sí funcionan.
- **`Array.pop()` devuelve el array mutado, no el elemento** (semántica no-JS):
  para usar el tope de una pila hay que hacer `a[a.length-1]` y luego `a.pop()`.
- **`int(string)` inválido**: ~~devuelve 0 en silencio~~ **CORREGIDO** — ahora
  `int("abc")`/`int("12x")` fallan con error claro (exit ≠ 0) en wasmtime;
  `int("12")` → 12. En wasmi sigue degradado a 0 (sin excepciones WASM).
- **Resolución de la ruta del `.dll` FFI**: el path de `extension` es relativo al
  **CWD**, no al archivo fuente (ni a la carpeta del módulo). Por eso la prueba
  (y los módulos de `/modules`) exigen correr desde la raíz del repo; los
  módulos FFI resuelven la ruta con la convención `modules/<mod>/<shim>.dll`
  + su `build.ps1`.

## Limitaciones / notas

- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): la llamada FFI del oráculo trapéa
  (`out of bounds memory access`) — es el bug pre-existente `host_call_wasmi`,
  no una regresión nueva. La prueba se valida con **wasmtime** (default).
- No se pudo obtener `mandelbrot.bf` desde la red; en su lugar se usa
  `torture.bf` (bucles anidados + 51000 bytes de salida determinista con wrap),
  que cubre el mismo eje (codegen de bucles/output) de forma reproducible.
- El oráculo C escribe su salida a `--out` (sin ensuciar stdout) y usa una tabla
  de saltos precalculada (misma semántica que el intérprete CLS, coste O(1) por
  bracket).
