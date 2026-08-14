# REPL interactivo

`clx repl` abre una sesión interactiva para evaluar código línea por línea con
el **intérprete JIT** (CLS → WASM → wasmtime), el mismo motor de `clx run`.

## Arranque

```
clx repl
```

Imprime el banner:

```
CLS 2.0 REPL (JIT) (Ctrl+C o :salir para salir)
```

El prompt es `> `. La sesión termina con `Ctrl+C`, EOF o uno de los comandos
de salida (imprime `Adiós!` al cerrar).

## Comandos de salida y ayuda

| Comando | Efecto |
|---|---|
| `exit`, `quit`, `:exit`, `:quit`, `:salir`, `:q` | Salir del REPL. |
| `:help`, `:h` | Muestra los comandos disponibles. |

## Evaluación

Cada línea se clasifica así:

- **Declaración**: si empieza con `var `, `const `, `function `, `for `,
  `while `, `if `, `switch `, `try `, `import `, `from `, `include `, `with `,
  `loop `, `return ` o `export `, se ejecuta como sentencia.
- **Expresión suelta**: cualquier otra línea se envuelve en `print(...)` y el
  valor resultante se imprime.

Si una sentencia no termina en `;` ni en `}`, el REPL agrega el `;` faltante.

```clx
> var x = 20
> x * 2
40
> function cuadrado(n: Int) -> Int { return n * n; }
> cuadrado(5)
25
> var saludo = "hola"
> saludo + " mundo"
hola mundo
```

Las sentencias se evalúan en un **entorno persistente**: las variables y
funciones declaradas quedan disponibles en las líneas siguientes. El estado se
transfiere entre instancias de WASM (globals + heap), así que los valores de
variables, arrays y strings sobreviven de una línea a la siguiente:

```clx
> var a = [1, 2, 3]
> a.push(4)
[1, 2, 3, 4]
> var i = 0
> while (i < 3) { i = i + 1; }
> i
3
```

Reasignar una variable declarada antes la actualiza:

```clx
> var x = 5
> x = 100
100
> x
100
```

## Motor

El REPL usa el **JIT** (`WasmBackend` + wasmtime, vía `cls-jit::repl`), el mismo
motor de `clx run`. La función `main` del usuario está reservada para el
intérprete del REPL (error si se declara). Cada línea se compila a un módulo
WASM nuevo que incluye las declaraciones anteriores (hoisted) y el código de la
línea actual.

### Limitaciones del JIT en el REPL

- Las funciones necesitan **tipos de retorno explícitos** (`-> Int`): sin la
  anotación el checker infiere `Void` y las llamadas devuelven `void`.
- Los parámetros de función necesitan **anotación de tipo** para aritmética:
  el checker exige tipos numéricos concretos para los operadores.
- `String + Int` (concatenación de tipos mixtos) no está soportado (igual que
  en archivos).
- Solo `wasmtime` (no wasmi).
- Los campos `static` de clases se re-inicializan en cada línea.
- El pool de strings del módulo vive bajo el heap (máx. 512 KB de datos de
  strings por sesión acumulada).

## Errores

Los errores de cada línea (lexer/parser/typeck/runtime) se muestran por stderr
con su contexto (código + caret) y la sesión continúa; la línea que falla no
deja estado nuevo.
