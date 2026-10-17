# Sintaxis y léxico

Referencia del léxico y la estructura básica del lenguaje. Fuentes:
`cls-core/src/frontend/lexer.rs`, `cls-core/src/frontend/token.rs` y los QA
validados en `examples/test/jit-test/units/`.

## Estructura de un programa

Un programa es una secuencia de declaraciones (funciones, clases, variables,
alias, etc.). Las declaraciones terminan en `;`. El `;` después de `}` de un
bloque es opcional.

```clx
# variables a nivel superior
var x: int = 42;

# punto de entrada obligatorio
function main(args: String[]) -> int {
    print("hola");
    return 0;
}
```

`main(args: String[]) -> int` es el punto de entrada; su valor de retorno es el
exit code del proceso.

## Identificadores

Estándar: letras, dígitos y `_`, no pueden comenzar con un dígito.

## Palabras clave (75)

| Categoría | Keywords |
|---|---|
| Declaración | `var`, `const`, `let`, `function`, `void`, `method`, `export` |
| Control de flujo | `if`, `elif`, `else`, `while`, `loop`, `for`, `each`, `in`, `when`, `switch`, `case`, `match`, `default` |
| Excepciones | `try`, `catch`, `finally` (el lanzamiento se hace con la intrinsic `throw(msg)`) |
| Funciones | `return`, `break`, `continue` |
| Tipos/OOP | `class`, `structure`, `interface`, `module`, `namespace`, `alias`, `enum` |
| Visibilidad | `public`, `private`, `protected`, `static`, `extends`, `is`, `super`, `readonly`, `me` |
| Módulos | `import`, `from`, `as`, `include` |
| Otros | `extension`, `async`, `await`, `sync`, `macro`, `global`, `config`, `then`, `and`, `or`, `not`, `true`, `false` |

Nota: `let` es un alias de `var`. `true`/`false` son keywords (no literales
reservados de otro tipo).

## Comentarios

Solo `#` hasta fin de línea. No existe `//` ni comentarios multilínea.

```clx
# esto es un comentario
var x = 1;  # comentario al final de línea
```

## Literales numéricos

Solo decimales.

- Enteros: `i64` (`42`, `-7`, `0`). No hay hex, binario, octal ni separador `_`.
- Floats: requieren punto decimal y/o notación científica (`3.14`, `1e300`,
  `2.5E-10`).

## Strings

Tres delimitadores producen el mismo tipo `String`: `"..."`, `'...'` y
`` `...` `` (backtick).

Escapes soportados: `\n`, `\t`, `\r`, `\b`, `\\`, `\'`, `\"`. Cualquier otro
`\x` se conserva literal.

Interpolación dentro de cualquier delimitador:

```clx
var nombre = "CLS";
var edad = 30;
print("Hola, $nombre");        # $var
print("Suma: ${2 + 3}");       # ${expr}
print(`Template $nombre ${edad + 1}`);
```

### Char

No existe literal `char`: `'a'` produce un `String` de un carácter. El token
`CharLiteral` existe en el lexer, pero nunca se produce. El tipo `Char` se
construye con los intrínsecos primitivos:

```clx
char(65);          # Char con el byte 65 ('A')
ord(char(65));     # 65 (código del byte, 0..255)

"A".toChar();      # Char[] = [65] (los BYTES del String)
char(200).toString();  # String de 1 byte con el valor crudo 200
```

**Modelo `bytes`/`str`:** `Char` es la **unidad de 1 byte** (0..255) para
buffers de bytes; `String` sigue siendo texto **byte-based** (`length`/`s[i]`
operan sobre bytes). Conviven como `bytes` y `str` en Python:

- `String.toChar() -> Char[]` — los bytes del string, uno por `Char`.
- `Char.toString() -> String` — String de 1 byte, **byte-transparente** (el byte
  crudo, NO el UTF-8 de un codepoint).
- `chars.join("")` — reconstruye el String byte a byte.
- `char(n: int) -> Char` (`n & 0xFF`) y `ord(c: Char) -> int` (0..255).

`'a'` sigue siendo un `String` (no un `Char`); para un byte suelto usar
`char(ord("a".toChar()[0]))` o directamente `"a".toChar()[0]`.

## Bools

`true` / `false` son keywords y valores del tipo `Bool`.

## Operadores

| Categoría | Operadores |
|---|---|
| Aritmética | `+`, `-`, `*`, `/`, `%`, `**` (potencia) |
| Comparación | `==`, `!=`, `<`, `<=`, `>`, `>=`, y keywords `is`, `in` |
| Lógicos | `&`, `\|`, `!` (se aceptan también `&&` y `\|\|`) |
| Asignación | `=`, `+=`, `-=`, `*=`, `/=`, `%=` |
| Incremento | `++`, `--` (pre y postfix) |
| Otros | `->` (retorno/flecha), `::`, `..`, `:` , `@`, `~`, `^`, `<<`, `>>`, `\` |

## Símbolos

`( ) [ ] { } , . ;` y `...` (ellipsis, token existente).

## CMX

Los tags `<tag>` se tokenizan aparte (lexer CMX), con desambiguación frente a
genéricos `<T>` y comparaciones `<` dentro de expresiones.

## Interacción con el JIT

`clx run` compila el archivo a WASM (ver `runtime/jit.md`). El JIT requiere
typecheck estricto: parámetros sin anotación de tipo, tipos `Any`/`Unknown` sin
anotar y arrays vacíos sin anotación generan error del emisor.
