# Prueba Shell POSIX — clsh (CLS) vs dash (oráculo externo)

Proyecto 2 de `examples/PLAN-PRUEBAS.md`. `clsh` es un intérprete de un
**subconjunto POSIX de shell** escrito en **CLS puro + FFI a C** (sin
`fs`/`http`/`json`/`strings` ni `host_calls`). Se compara **byte a byte**
(stdout, stderr y exit code) contra **`dash`**, el oráculo externo
independiente del lenguaje.

## Estructura

```
examples/clsh/
├── cls.json                 # { name, version, entry, pathModules }
├── src/
│   ├── main.clsx            # CLI: fija fd 0/1/2 en binario, lee el .sh (FFI) y lo corre
│   ├── ffi.clsx             # puente FFI (único acceso al SO): I/O de archivos, cwd, fd
│   ├── lexer.clsx           # tokenizador (operadores, comillas, escapes, $())
│   └── shell.clsx           # intérprete: expansión, builtins, control, pipes, redirs
├── oracle/
│   ├── clsh_io.c            # puente C (open/read/write/chdir/stat) — tipos sancionados FFI
│   └── clsh_io.dll          # compilado (gcc -shared)
├── tests/*.sh               # corpus canónico
├── tests/*.expect           # stdout esperado (generado por dash)
├── tests/*.expect.err       # stderr esperado
├── tests/*.expect.exit      # exit code esperado
├── findings/                # repros mínimos de regresiones (ver findings/README.md)
└── run.ps1                  # runner diferencial (genera .expect y compara)
```

## Cómo correr

Desde la **raíz del repo** (CWD = raíz; el `.dll` de FFI se resuelve relativo al
CWD):

```powershell
# un script suelto (ojo: los args de la app van tras `--`)
target\debug\clx.exe run examples\clsh\src\main.clsx -- examples\clsh\tests\echo.sh

# toda la suite (compila el oráculo si falta, regenera .expect, compara, exit!=0 si diverge)
powershell -File examples\clsh\run.ps1
```

> **Nota sobre args:** `clx run <entry> <scripts...>` NO pasa los posicionales a
> `main(args)`. Hay que usar el separador `--`:
> `clx run examples\clsh\src\main.clsx -- <script.sh> [<script2.sh> ...]`.
> (`main.clsx` toma `args[0]` como ruta del script; a futuro: varios scripts.)

## Oráculo

- **`dash`**: `c:\devkitPro\msys2\usr\bin\dash.exe` (principal).
- Alternativas (si falta `dash`): `C:\Program Files\Git\usr\bin\sh.exe`
  (`run.ps1` las prueba en ese orden).
- Normalización documentada: se convierte **CRLF → LF** en ambos lados antes de
  comparar (dash en MSYS emite `\n`; clsh fija los fds en binario pero se
  normaliza por robustez).

## Corpus (categorías)

| Test | Eje |
|------|-----|
| `echo`, `escape` | `echo` con/sin `-n`, comillas simples/dobles, escapes `\` |
| `vars`, `expand2`, `assign_only` | `$x`, `${x}`, `$?`, asignación (incl. múltiple `a=1 b=2`) |
| `status`, `andor` | `$?` tras comandos, `;`, `&&`, `\|\|` |
| `test` | `test` / `[ ]`: `-eq -ne -lt -le -gt -ge -z -n = !=` |
| `control`, `nested_if`, `if_cmd` | `if/elif/else`, `while`, `for ... in` |
| `cd` | `cd .` / `cd <dir>` y `$?` (sin `pwd`, por el formato de ruta) |
| `subst`, `cmdsubst2`, `for_cmdsubst` | `$(cmd)`, anidada, en `for` |
| `bytes` | UTF-8 multibyte/emoji idéntico a dash |
| `redirect` | `>`, `>>`, `<`, `2>`, `$?` |
| `comment`, `empty` | comentarios `#`, líneas vacías |
| `pwd` | *(excluido del diff: el CWD se imprime con separadores distintos en MSYS vs win32)* |
| `pipe`, `subshell`, `read2` | **divergencias conocidas** (ver `findings/`) |

## Resultado

`powershell -File examples\clsh\run.ps1`:

```
  [ OK ] andor ... vars           (22/25 byte-identicos a dash)
  [FAIL] pipe, read2, subshell    (3 divergencias conocidas, ver findings/)
RESULTADO: 3/25 test(s) DIVERGEN de dash   (exit 1)
```

- **22/25** scripts son **byte-idénticos** a dash (stdout + stderr + exit).
- Las 3 divergencias son de **comportamiento del shell** (no del lenguaje):
  aislamiento de subshell/pipe (falta de procesos) y `read` multi-variable.

## Regresiones del lenguaje detectadas (bloqueantes)

La construcción del shell destapó **4 regresiones del lenguaje/JIT**
(detalle y repros en `findings/README.md`):

| # | Feature | Síntoma | Estado en el scaffold |
|---|---------|---------|-----------------------|
| 1 | literal `"…$"` (termina en `$`) | **cuelga el compilador** (parseo/typeck) | rodeado (`ffi::dollar()`) |
| 2 | `>` `<` `>=` `<=` de `String` | resultado **invertido** | rodeado (`contains`) |
| 3 | `var` en bloques: scope plano por función | error "ya declarado" | renombrado |
| 4 | FFI retorno `CString` sin param `CString` | error de tipo espurio | param dummy |

## Reglas de autocontención

- Todo el I/O pasa por `oracle/clsh_io.c` vía `extension` (FFI). **No** se usan
  `fs`/`http`/`json`/`strings`.
- **No** se exponen punteros crudos: solo `CString`/`CInt`/`CLong`.
- El runtime del shell es CLS puro (`lexer.clsx`, `shell.clsx`).

## Compilar el oráculo C

```powershell
gcc -shared -O2 -o examples\clsh\oracle\clsh_io.dll examples\clsh\oracle\clsh_io.c
```

`run.ps1` lo recompila automáticamente si el `.c` es más nuevo que el `.dll`.
