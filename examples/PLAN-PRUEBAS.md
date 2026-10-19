# Plan de pruebas de CLS 2.0 (proyectos en `examples/`)

> Sin tree-walker no hay oráculo interno. Cada prueba define **su oráculo
> externo** y deja claro qué bug del JIT detecta. Esta es la **redacción previa**
> (revisar antes de codear). El servidor HTTP queda **excluido**.

---

## 0. Estado del entorno (condiciona el diseño)

| Hecho verificado | Implicación |
|---|---|
| `optimizationLevel` existe en `cls.json` pero **no se usa** en ningún crate; `clx build` no acepta `-O0/-O2` (solo `-o`). | El **oráculo 3 (O0 vs O2) NO está disponible hoy**. Alternativas abajo. |
| Módulos `os`, `path`, `process`, `time`, `random` **eliminados** en dev-2. | Procesos/tiempo/sockets/paths = **FFI** (`extension` + `when (os:)`). |
| FFI soportado solo en JIT; libs `libc`/`libm` mapeadas; **≤16 args**; `CString/CArray/CRecord/CStruct/Struct(...)` + `structure`. | Suficiente para oráculos C, pero requiere compilar DLLs por SO. |
| `clx` corre en **win32** (esta máquina). | Muchos oráculos CLI (`dash`, `dig`, `sha256sum`, `guile`) no están en Windows por defecto. |
| `clx test` (+`testEntry`) y `clx run` (JIT) disponibles. | Runner de las pruebas. |
| Intrínsecos de test (`test()`, aserciones) **aún no existen** (fase final). | Los checks se hacen con `assert` propio / exit codes hasta que existan. |

### Oráculos disponibles

1. **C vía FFI** — diferencial in-process (el más fuerte; reemplaza al walker).
2. **Especificaciones/CLI externas** — `dig`, `sqlite3`, `sha256sum`/`certutil`,
   `dash`/`bash`, `guile`/`sbcl`, intérprete BF en C.
3. **JIT vs JIT** — **no** por O0/O2. Alternativas viables hoy:
   - **caché on/off** (`clx clean` entre corridas) — detecta no-determinismo / bugs de caché;
   - **wasmtime vs wasmi** (`--wasmi`) — detecta discrepancias de codegen/host;
   - **`CLS_JIT_TIMING=1`** — solo perf.

### Decisión transversal pendiente
¿Las suites corren en **Windows** (equivalente por oráculo: `certutil -hashfile`,
`nslookup`, `sqlite3.exe`, PowerShell) o en **Linux/CI** (`dash`, `dig`,
`sha256sum`, `guile`)? El plan asume una abstracción `oracle(name)` por SO.

---

## 1. Suite de conformidad + differential testing vs C  ·  `examples/conformance/`

**Oráculo:** C vía FFI (in-process) + specs.

**Qué prueba:** *todo* el lenguaje de forma sistemática y reproducible.

**Estructura**
```
examples/conformance/
├── cls.json
├── src/
│   ├── main.clsx              # runner: descubre casos y compara
│   ├── difftest.clsx          # pares CLS<->C por feature
│   └── cases/                 # un .clsx por feature (ver abajo)
├── fuzz/
│   ├── gen.ps1                # genera N expresiones aleatorias
│   └── run.ps1                # escribe .clsx temporal + corre + compara
└── oracle/
    ├── oracle.c               # evaluador C de referencia
    └── oracle.dll (build)     # extension "..." as C { ... }
```

**Casos de prueba (pares CLS ⇄ C)**
- **Aritmética**: `+ - * / %`, precedencia, asociatividad, negativo/unario.
- **Promoción int/float**: `int/int`, `int/float`, mezcla y redondeo.
- **Overflow**: `i64`/`i32` envolvente vs C (definir semántica; CLS ¿wrapping?).
- **Bitwise**: `& | ^ ~ << >>`, shifts ≥ ancho, signo.
- **Strings**: `length`, `indexOf`, `slice`, `split`, `contains`, `startsWith`,
  `endsWith`, concat, con **bytes UTF-8 multibyte** (vs `strlen`/`memcmp` en C).
- **Comparaciones**: `< <= > >= == !=` en int/float/bool/String.
- **Control**: `if/while/for/for-each`, `break/continue`, `return` anidado.
- **Funciones/closures**: recursión, captura, paso de función (indirect call).
- **Arrays/records/tuplas**: index, length, mutación, spread.
- **Excepciones**: `throw/try/catch/finally` — comparar flujo (C simulado con `setjmp`).
- **Fuzzer**: driver genera expresiones aritmético-lógicas aleatorias con
  enteros tipados; el oráculo C las evalúa con la misma semántica.

**Cómo detecta bugs:** precedencia/asociatividad, promoción, overflow, shifts,
bytes UTF-8, y **divergencia CLS↔C** en cientos de expresiones generadas.

**Esfuerzo:** alto. **Prioridad:** bloqueante.

---

## 2. Shell POSIX  ·  `examples/clsh/`

**Oráculo:** `dash`/`bash` (modo `--diff` byte a byte de stdout/stderr/exit).

**Qué prueba:** lexer/parser propios, strings bytes, VFS, y FFI de procesos
(`fork`/`execve`/`waitpid` en POSIX; equivalente Windows vía FFI).

**Estructura**
```
examples/clsh/
├── cls.json
├── src/main.clsx        # REPL + interprete de scripts
├── tests/*.sh           # scripts canonicos
├── tests/*.expect       # stdout/stderr/exit (generado con dash)
└── diff.ps1 / diff.sh   # corre en clsh y en dash, diff
```

**Casos**
- Builtins: `echo`, `cd`, `pwd`, `exit`, `export`, `read`, `test`/`[`.
- Pipes `|`, redirecciones `>` `>>` `<` `2>`, `&&`, `||`, `;`.
- Expansión: variables `$x`, `$?`, comillas simples/dobles, `$(cmd)`.
- `for`/`while`/`if`; subshells.
- Bytes: un `echo -n 'á😀'` debe salir **idéntico** a dash (valida el fix de bytes).

**Cómo detecta bugs:** si `clsh --diff` y `dash` difieren en cualquier byte o
exit code → bug (parser, expansión, o marshalling `char**` en FFI).

**Riesgo:** `fork/execve` no existe en Windows ⇒ este proyecto es **Linux-first**
(aunque el intérprete en sí es portable). **Esfuerzo:** medio. **Prioridad:** alta.

---

## 3. Brainfuck + benchmark  ·  `examples/brainfuck/`  ⬅️ **empezar aquí**

**Oráculo:** intérprete BF en C (~100 LOC) vía FFI + outputs canónicos
(`mandelbrot.bf`, `bench.bf`).

**Qué prueba:** el codegen del JIT bajo millones de iteraciones; índices de
array, `while`, ASCII/bytes, y perf.

**Estructura**
```
examples/brainfuck/
├── cls.json
├── src/bf.clsx            # interprete (CLS puro) + runner
├── src/bf_c.clsx          # variante que llama a C (extension)
├── oracle/bf.c, bf.dll
├── programs/*.bf, *.out   # mandelbrot.bf + salida esperada
├── diff.ps1               # corre CLS y C, compara bytes + tiempo
└── benchmark.clsx
```

**Casos**
- `hello.bf`, `mandelbrot.bf`, `bench.bf` (loop caliente), `cell.bf` (255 wrap).
- Tape de 30k celdas; input `,`/output `.` byte a byte.
- Comparación **CLS vs C** (output + tiempo) y CLS con **caché on/off**.

**Cómo detecta bugs:** cualquier error de codegen (bounds, aritmética de
punteros del tape, branch mal emitido) rompe `mandelbrot`; el tiempo revela si el
JIT realmente compila o interpreta.

**Esfuerzo:** bajo. **Prioridad:** alta (mejor ROI). **Nota:** escribir *primero*
esto antes de los proyectos grandes.

---

## 4. *(excluido: servidor HTTP)*

---

## 5. SQLite ORM  ·  `examples/sqlite-orm/`

**Oráculo:** `libsqlite3` vía FFI **y** el CLI `sqlite3` (doble).

**Qué prueba:** FFI inverso (callbacks C→CLS), strings/encoding, structs/records
en ABI C, y generación de SQL.

**Estructura**
```
examples/sqlite-orm/
├── cls.json
├── src/orm.clsx           # query builder + CRUD
├── src/ffi.clsx           # extension sqlite3.dll / libsqlite3.so
├── tests/*.clsx
└── verify.ps1             # compara .dump del ORM vs insert via sqlite3 CLI
```

**Casos**
- Insertar 10k filas vía ORM y las mismas vía `sqlite3` CLI; `diff` de `.dump`.
- Tipos: `int`, `float`, `text` (UTF-8), `blob`, `NULL`.
- Vacío/orden/paginación; transacciones; callbacks (`sqlite3_exec`) C→CLS.
- `PRAGMA integrity_check` = `ok` en ambas.

**Cómo detecta bugs:** divergencia de tipos/NULL/orden/encoding; caída en el
**FFI inverso** (punto más frágil).

**Esfuerzo:** alto. **Prioridad:** alta. **Requisito:** `sqlite3` CLI o DLL.

---

## 6. DNS resolver  ·  `examples/dns/`

**Oráculo:** `dig` / `nslookup` + RFC 1035.

**Qué prueba:** bit manipulation, endianness, sockets UDP (FFI), parseo binario.

**Estructura**
```
examples/dns/
├── cls.json
├── src/resolver.clsx      # build packet + parse response
├── src/udp.clsx           # extension ws2_32 / libc sockets
├── tests/*.clsx
└── diff.ps1               # vs dig/nslookup
```

**Casos**
- A/AAAA/MX/TXT/CNAME de dominios estables; TTL (ignorar valor, validar forma).
- Compresión de nombres (punteros `0xC0`), respuesta truncada + TCP retry.
- Servidor autoritativo local en puerto alto (recomendado: **sin red externa**).
- Comparar respuesta con `nslookup`/`dig +short` (o un servidor propio).

**Cómo detecta bugs:** endianness, shifts, límites; `nslookup` da oráculo externo.

**Esfuerzo:** medio. **Prioridad:** media. **Riesgo:** red externa inestable ⇒
usar servidor autoritativo propio.

---

## 7. Motor de plantillas CMX + golden files  ·  `examples/cmx-templates/`

> **Estado: implementado (reconstruido tras el rework CMX).** Renderer CMX→HTML
> en CLS puro (`src/engine.clsx`) + golden files (16 páginas) + oráculo cruzado
> Jinja2 (paridad) + findings. Ver `examples/cmx-templates/README.md`.
> Resueltos: `tag` ya es String (componente en `tagCallable`/`isComponent`),
> atributo desnudo → `true`, literal CMX dentro de `{...}` parsea. GAP: local
> `Value` sin tag (GAP 19).

**Oráculo:** golden files a mano (`tests/golden/*.html`) + Jinja2 (Python) donde
solape.

**Qué prueba:** CMX (la feature más nueva), la clase core `CMX` (métodos
`isPrimitive/isEmpty/isElementOf/flattenChildren/tryCallTag/toString/cloneWith`),
`tag`/`tagCallable`/`isComponent`, `render()` en CLS puro, escape HTML,
interpolación, componentes (tag mayúscula), spread de props, shorthand, atributos
desnudos, anidado en `{...}` y `CMX<P, PP>` borrado.

**Estructura**
```
examples/cmx-templates/
├── cls.json
├── src/engine.clsx        # e(), raw(), render(), attrs
├── src/components.clsx    # layout, navbar, card, badge, todoList
├── src/pages.clsx         # vistas (un eje por página)
├── tests/golden/*.html    # salida esperada versionada
├── tests/oracle/*.py      # oráculo cruzado Jinja2
└── tests/findings/*.clsx  # repros (exit != 0 = bug)
```

**Casos**
- Escape: `<`, `>`, `&`, `"`, `'` (y **no** doble-escape).
- Loops vía `renderEach`, condicionales `{c ? a : b}`, layouts por composición.
- Componentes: `isComponent` + `tagCallable(props, children)` (o `tryCallTag`).
- `raw(...)` sin escape; props ordenadas; atributos desnudos/booleanos.
- Clase `CMX`: predicados, `toString`, `cloneWith`, `is Cmx`, `CMX<P,PP>` borrado.
- Multibyte/emoji; spread `{...base}`; shorthand `{var}`; CMX anidado en `{...}`.

**Cómo detecta bugs:** cualquier cambio de CMX/runtime rompe un golden; fuerza a
versionar la salida esperada (sin walker, el golden **es** el oráculo).

**Esfuerzo:** medio. **Prioridad:** media.

---

## 8. Gestor de paquetes + verificación SHA-256  ·  `examples/pkgcheck/`

> **Estado: implementado.** SHA-256 (FIPS 180-4) en CLS puro (0 FFI) validado
> contra `hashlib` (205 digests: NIST + 1e6'a' + 200 LCG) y `certutil` (archivo).
> Ver `examples/pkgcheck/README.md`.

**Oráculo:** `hashlib` (Python) + `certutil -hashfile SHA256` (Windows).

**Qué prueba:** aritmética entera de 32 bits (bitwise/shifts/overflow), y —de
paso— el dogfooding de módulos y del nodo `fs`.

**Estructura**
```
examples/pkgcheck/
├── cls.json
├── verify.ps1             # main.clsx + oraculo hashlib + certutil
├── src/sha256.clsx        # SHA-256 puro en CLS
├── src/hashfile.clsx      # sha256 de un archivo (nodo fs)
├── src/main.clsx          # 205 digests + corrupt=
└── tests/oracle.py        # oraculo hashlib
```

**Casos**
- Vectores NIST ("" , "abc", 448-bit, 896-bit, 1M 'a') + 200 inputs aleatorios
  (LCG determinista idéntico en CLS y Python).
- Cross-check de archivo con `certutil` (independiente de Python).
- Integridad: detectar corrupción (1 bit) → hash distinto.

**Cómo detecta bugs:** shifts rotativos, overflow (máscara 32-bit), operadores
bitwise. Si el SHA-256 en CLS ≠ `hashlib`/`certutil`, el gestor sería inútil.

**Esfuerzo:** alto. **Prioridad:** media. **Nota:** el SHA-256 de `clx` hoy es en
Rust (`packages.rs`) — este proyecto porta el algoritmo a CLS.

---

## 9. Snake / Tetris en terminal  ·  `examples/snake/`

**Oráculo:** **grabaciones de input** reproducidas (estado final determinista) —
sin oráculo externo automático.

**Qué prueba:** FFI de structs (`termios`/winapi), `VFS tmp://`, arrays, estado de
juego, y el bucle principal caliente.

**Estructura**
```
examples/snake/
├── cls.json
├── src/game.clsx          # lógica pura (update(state, input) -> state)
├── src/term.clsx          # extension termios/winapi (raw mode, colores)
├── src/main.clsx          # loop + render
└── replays/*.txt, *.final # secuencia de inputs + estado final esperado
```

**Casos**
- Update de lógica es **puro** ⇒ testeable con `replays`.
- Colisión, wrap, crecimiento, game-over; pausa/reinicio.
- Tamaño de terminal (SIGWINCH/consulta), sin crash en borde.

**Cómo detecta bugs:** solo "no crashea" + FFI de structs; el replay da
determinismo. **Esfuerzo:** bajo. **Prioridad:** baja.

---

## 10. Intérprete Scheme (subconjunto)  ·  `examples/scheme/`

> **Estado: implementado.** Intérprete Scheme (subconjunto) en CLS puro: reader
> de S-expresiones + evaluador con entornos/closures/recursión. 7 programas
> verificados contra salidas esperadas (y guile opcional). Ver
> `examples/scheme/README.md`.

**Oráculo:** salidas `.out` versionadas (semántica estándar) + `guile` (opcional).

**Qué prueba:** compilador CLS con un workload real — parser, clases
autorreferentes, funciones como valor, closures, recursión, arrays, strings.

**Estructura**
```
examples/scheme/
├── cls.json
├── src/main.clsx      # Val + Reader + Interp + main
├── tests/*.scm        # programas Scheme
├── tests/*.out        # salida esperada
└── diff.ps1           # CLS vs .out (+ guile opcional)
```

**Casos**
- `+ - * /`, comparaciones, `display`/`write`/`newline`.
- `define`, `lambda`, `let`, `if`, `cond`, `and`, `or`, `set!`, `quote`.
- Listas (`cons/car/cdr/list/append/length`), recursión, `fib`, Ackermann.
- Closures y orden superior (`make-adder`, `compose`, `twice`, `map`).

**Cómo detecta bugs:** cualquier divergencia del intérprete CLS respecto de la
semántica Scheme esperada (o guile) señala un bug del compilador/runtime.

**Esfuerzo:** alto. **Prioridad:** baja. **Nota:** intérprete tree-walking (no
bytecode/VM); sin TCO ni floats (ver README).

**Esfuerzo:** alto. **Prioridad:** baja (sube si sobra tiempo).

---

## Tabla final

| # | Proyecto | Carpeta | Oráculo | Esfuerzo | Prioridad | SO |
|---|----------|---------|---------|----------|-----------|----|
| 1 | Conformidad + fuzzer | `conformance/` | C (FFI), specs | Alto | **Bloqueante** | cross |
| 3 | Brainfuck + bench | `brainfuck/` | BF en C | **Bajo** | **Alta** (ROI) | cross |
| 2 | Shell POSIX | `clsh/` | dash/bash | Medio | Alta | Linux-first |
| 5 | SQLite ORM | `sqlite-orm/` | libsqlite3 + CLI | Alto | Alta | cross* |
| 6 | DNS | `dns/` | dig/nslookup | Medio | Media | cross |
| 7 | CMX templates | `cmx-templates/` | golden files | Medio | Media | cross |
| 8 | Gestor + SHA-256 | `pkgcheck/` | sha256sum/certutil | Alto | Media | cross |
| 9 | Snake/Tetris | `snake/` | replays | Bajo | Baja | cross |
| 10 | Intérprete Scheme | `scheme/` | .out (+guile) | Alto | Baja | cross |

`*` requiere `sqlite3`/DLL por SO.

## Ideas de implementación (a validar con las reglas)
- Un módulo común `examples/_lib/` con `assert()`-propio, `checkEq`, y un
  `oracle(name)` que resuelva el binario por SO.
- Cada proyecto: `cls.json` con `testEntry`, un `run.ps1`/`run.sh`, y README con
  su oráculo.
- Prioridad de ejecución sugerida: **3 (una tarde) → 1 (infraestructura) → resto**.
