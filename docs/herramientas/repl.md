# REPL interactivo

`clx repl` abre una sesión interactiva para evaluar código línea por línea.

## Arranque

```
clx repl
```

Imprime el banner:

```
CLS 2.0 REPL (Ctrl+C o :salir para salir)
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
> function cuadrado(n: int) -> int { return n * n; }
> cuadrado(5)
25
```

Las sentencias se evalúan en un entorno persistente: las variables y funciones
declaradas quedan disponibles en las líneas siguientes.

## Motor

El REPL usa el **tree-walker** (`Interpreter`) con la **stdlib core**
(`math`, `json`, `async`). No carga los módulos del nodo desktop (`fs`,
`http`, `os`, `path`, `process`, `time`, `random`, `Lib`).

Los errores de cada línea (lexer/parser/runtime) se muestran por stderr y la
sesión continúa.