# Regresiones y divergencias halladas por la prueba Shell POSIX (clsh)

Repros mínimos y autocontenidos de los fallos detectados al construir
`examples/clsh/` (intérprete de un subconjunto POSIX de shell en CLS, comparado
**byte a byte** contra `dash`). Los `01..04` son del **lenguaje/JIT**; los
`05..07` son del **comportamiento del shell** (arquitectura/implementación).

Todos los comandos se ejecutan **desde la raíz del repo** (el `.dll` de FFI se
resuelve relativo al CWD):

```
target\debug\clx.exe check examples\clsh\findings\<archivo>.clsx
target\debug\clx.exe run   examples\clsh\findings\<archivo>.clsx
```

---

## 1. Un literal de String que TERMINA en `$` cuelga el compilador  ·  CRÍTICO

- **Archivo:** `01-dollar-string-hang.clsx`
- **Feature:** lexer/parser de CLS (string literals / `$`).
- **Repro:**
  ```
  target\debug\clx.exe check examples\clsh\findings\01-dollar-string-hang.clsx
  ```
- **Esperado:** compila y corre (imprime `$`).
- **Obtenido:** **cuelga indefinidamente** (CPU 100 %, sin salida). Afecta a
  `clx check` y a `clx run` por igual (parseo/typeck).
- **Caracterización:** el disparador es un `$` en la **última** posición antes de
  la comilla de cierre:

  | fuente    | resultado |
  |-----------|-----------|
  | `"$"`     | cuelga    |
  | `"x$"`    | cuelga    |
  | `"$$"`    | cuelga    |
  | `"\$"`    | cuelga    |
  | `"$x"`    | ok        |
  | `"ab$cd"` | ok        |
  | `"${x}"`  | ok        |

- **Impacto:** **bloquea todo el intérprete shell**, cuya expansión necesita el
  literal `$` (comparar el byte `$`). Fue la causa de que el scaffold no
  compilara. Workaround aplicado: obtener `$` con `ffi::slice("$ ", 0, 1)` (ver
  `src/ffi.clsx: dollar()`).
- **Causa probable:** el manejo de interpolación `$` (lexer/parser) no avanza el
  cursor cuando `$` está al final del literal → bucle infinito.

## 2. Operadores relacionales de String invertidos (`>`, `<`, `>=`, `<=`)  ·  ALTO

- **Archivo:** `02-string-relational-inverted.clsx`
- **Feature:** operadores binarios de comparación sobre `String`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\clsh\findings\02-string-relational-inverted.clsx
  ```
- **Esperado:** `x>a=T  a<x=T  b>=a=T  x>=a=T` (exit 0).
- **Obtenido:** `x>a=F  a<x=F  b>=a=F  x>=a=F` (exit 1).
- **Caracterización:** `>` se comporta como `<` y viceversa; `>=` como `<=` y
  viceversa. `==`/`!=` dan el resultado correcto **salvo** con cadena vacía
  (ver §8, ya corregido).

  | expr          | esperado | obtenido |
  |---------------|----------|----------|
  | `"x" > "a"`   | T        | F        |
  | `"x" < "a"`   | F        | T        |
  | `"a" > "x"`   | F        | T        |
  | `"a" < "x"`   | T        | F        |
  | `"a" >= "b"`  | F        | T        |
  | `"b" >= "a"`  | T        | F        |
  | `"a" >= "a"`  | T        | T        |
  | `"ab" >= "aa"`| T        | F        |

- **Impacto:** cualquier clasificación por rango de caracteres (`c >= "a"`, etc.)
  da resultados invertidos. Rompió `isNameStart`/`isNumber` del shell (detección
  de asignaciones y de nombres de variable). Workaround: usar `String.contains`
  sobre el alfabeto (`"abc...".contains(c)`).
- **Causa probable:** en el emisor WASM, `>`/`>=` sobre `String` (i64 empaquetado
  `ptr<<32|len`) se emiten con el predicado equivocado (se comparan los
  punteros/len en orden invertido).

## 3. El cuerpo de una función es UN SOLO scope para `var`  ·  MEDIO

- **Archivo:** `03-var-flat-scope.clsx`
- **Feature:** typeck / resolución de variables (`var` redeclarado en bloques).
- **Repro:**
  ```
  target\debug\clx.exe check examples\clsh\findings\03-var-flat-scope.clsx
  ```
- **Esperado:** compila (cada bloque `{ }` es un scope).
- **Obtenido:** ERROR `El nombre 'x' ya está declarado en este scope`.
- **Causa (analizada):** `TypeChecker::check_block` (`typeck/statements.rs:155`)
  **no** hace `push_scope()`; el chequeo de duplicados
  (`typeck/decls.rs:57`) usa `self.scopes.last()` (el scope de la función) para
  todo el cuerpo, así que dos `var x` en bloques hermanos o anidados colisionan.
- **Impacto:** impide escribir código con nombres locales repetidos en bloques
  distintos (muy común). El scaffold del shell tuvo ~6 colisiones que hubo que
  renombrar (`fd`, `name`, `j`, `r`, `op`, …).

## 4. FFI: retorno `CString` no asignable a `String` salvo que haya un param `CString`  ·  MEDIO

- **Archivo:** `04-ffi-cstring-ret.clsx`
- **Feature:** typeck de `extension` (FFI) — inferencia del tipo de retorno nativo.
- **Repro:**
  ```
  target\debug\clx.exe run examples\clsh\findings\04-ffi-cstring-ret.clsx
  ```
- **Esperado:** compila (`clsh_getcwd` es `char*` → `String`).
- **Obtenido:** ERROR (exit 1) `Tipo de retorno CString no coincide con el declarado String`.
  (`clx check` lo marca solo como WARN → exit 0; el JIT `clx run` es estricto.)
- **Caracterización:**
  - native `() -> CString`  **sin** params        → retorno inferido `CString` → **error**;
  - native `(x: CString) -> CString`               → retorno inferido `String`  → **ok**;
  - native `(x: CInt) -> CString`                  → retorno inferido `CString` → **error**.
  - native `(x: CString) -> CInt`                  → retorno inferido `String` (¡no Int!).
  La presencia de un parámetro `CString` cambia la inferencia de **todo** el
  retorno. `map_native_ret` (`typeck/types.rs:107`) no mapea `CString`.
- **Impacto:** bloqueó el scaffold (todas las funciones FFI de I/O devuelven
  `CString`); se rodeó con un parámetro `CString` dummy en las funciones de 0
  params (`clsh_getcwd`, `clsh_read_line`).

## 8. `==` / `!=` de String con cadena VACÍA da falso positivo  ·  CORREGIDO

- **Archivo:** `08-empty-string-eq.clsx`
- **Feature:** igualdad estricta (`==`/`!=`) sobre `String`.
- **Repro:**
  ```
  target\debug\clx.exe run examples\clsh\findings\08-empty-string-eq.clsx
  ```
- **Esperado:** `e==s: false` (exit 0).
- **Obtenido (bug):** `e==s: true`, `e!=s: false`.
- **Causa raíz:** `__intr_str_eq` (`cls-internals/wasm/src/strings.rs:361`) y su
  fallback `host_str_eq` (`cls-jit/src/host.rs:605`) devolvían `1` cuando
  `a_ptr == b_ptr` **antes** de mirar la longitud. Un literal de 0 bytes (`""`)
  no avanza el cursor del pool, así que comparte puntero con el literal
  siguiente → falso positivo.
- **Fix:** comparar **longitud primero** (si difieren → `0`) y usar el atajo de
  puntero solo cuando las longitudes coinciden.
- **Estado:** corregido (R5). `""==""`→`T`, `""!="x"`→`T`, `"x"==""`→`F`,
  `"ab"=="ab"`→`T`, `"ab"=="abc"`→`F`, UTF-8 iguales/distintas → OK.

---

## Divergencias del SHELL (no del lenguaje)

Estas no son bugs de CLS sino del intérprete shell construido (arquitectura sin
procesos). Quedan **fallando a propósito** en `run.ps1` para señalarlas.

## 5. `read a b` no asigna el resto al último nombre  ·  `05-read-multi-var.sh`

- POSIX reparte la línea: el último nombre toma el resto sin re-split.
- clsh reparte un campo (ya dividido por IFS) por nombre.
- `dash: x=one / y=two three` vs `clsh: x=one / y=two`.
- Causa: `Sh::biRead` (`src/shell.clsx`) asigna `f[k-1]` a cada nombre.

## 6. `( ... )` no aísla el estado  ·  `06-subshell-not-isolated.sh`

- `Sh::run_subshell` reutiliza el mismo `Sh`, así que `( x=inner )` filtra `x`.
- `dash: inner / outer` vs `clsh: inner / inner`.
- **Bloqueo de lenguaje:** CLS no tiene procesos (`fork`/`CreateProcess` sin
  captura). Para un subshell real haría falta FFI de procesos (posible en POSIX,
  no intentado aquí) o un modelo de scopes separado.

## 7. Pipe: `read` del lado derecho afecta al shell padre  ·  `07-pipe-read-state.sh`

- `run_pipeline` captura el stdout del izquierdo en `me.input` y corre el derecho
  en el mismo `Sh`; `read` escribe en el padre.
- `dash: x=` (vacío) vs `clsh: x=hello`.

### `System` (comandos externos) — sondeo

Se probó FFI a `msvcrt.dll` → `system()` y **funciona** (ejecuta el comando; su
salida va al stdout real). Pero:
- corre vía `cmd.exe` (semántica distinta de POSIX: CRLF, builtins propios);
- **no es capturable**: `_popen`+`fgets` necesita un buffer de bytes
  (allocator/`CArray` con puntero crudo), que CLS no expone → **bloqueado**.
Por eso el corpus se limita a **builtins + expansión** (que ya cubre el eje:
lexer/parser/expansión/bytes/exit).
