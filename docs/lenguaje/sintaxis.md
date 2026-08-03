# Sintaxis del lenguaje

## Comentarios

Los comentarios de línea usan `#`:

```
# esto es un comentario
var x = 1;   # comentario al final de la línea
```

## Identificadores

Los identificadores comienzan con una letra o `_` y pueden contener letras,
dígitos y `_`. Son sensibles a mayúsculas.

## Palabras reservadas

`if`, `elif`, `then`, `else`, `while`, `loop`, `for`, `each`, `switch`, `case`,
`default`, `try`, `catch`, `finally`, `throw`, `return`, `break`, `continue`,
`with`, `in`, `is`, `and`, `function`, `fun`, `void`, `method`, `var`, `const`,
`let`, `class`, `structure`, `interface`, `module`, `namespace`, `alias`,
`enum`, `import`, `from`, `as`, `include`, `export`, `public`, `private`,
`protected`, `static`, `readonly`, `global`, `async`, `await`, `sync`, `macro`,
`extends`, `me`, `super`, `config`, `true`, `false`, `null`.

## Literales

| Literal | Ejemplo |
|---------|---------|
| Entero | `42`, `-1`, `0xFF` (no soportado aún) |
| Flotante | `3.14`, `-0.5` |
| Cadena | `"texto"`, `'texto'`, `` `plantilla` `` |
| Booleano | `true`, `false` |
| Carácter | `'a'` |
| Nulo | `null` |

### Interpolación de cadenas

Las cadenas con comillas dobles y backtick admiten interpolación:

```
var nombre = "CLS";
print("Hola, $nombre");            // variable
print("Suma: ${2 + 3}");           // expresión
```

## Variables

```
var x: int = 1;        // declaración tipada
var y = 2;             // inferencia de tipo
const PI = 3.14;       // constante (infiere literal type)
let z = 0;             // alias de var
```

- `const` no puede reasignarse.
- `var` infiere el tipo base de su inicializador.
- `const` infiere un *literal type* (por ejemplo `"constante"`), útil para
  anotaciones y uniones.

## Tipos básicos

| Tipo | Descripción |
|------|-------------|
| `Int` / `int` | Entero de 64 bits con signo. |
| `Float` / `float` | Punto flotante de 64 bits. |
| `String` / `str` | Cadena UTF-8. |
| `Bool` / `bool` | Booleano. |
| `Char` / `char` | Carácter Unicode. |
| `Any` | Tipo dinámico. |
| `Null` | El valor nulo. |
| `Void` | Sin valor (retorno de procedimientos). |

## Operadores

### Aritméticos

`+`, `-`, `*`, `/`, `%` (módulo), `**` (potencia).

### Comparación

`==`, `!=`, `<`, `<=`, `>`, `>=`. `is` valida instancia de clase/enum.

### Lógicos

`&&`, `||`, `!`, `and`. El cortocircuito funciona: `false && expr` no evalúa
`expr`.

### Asignación

`=`, `+=`, `-=`, `*=`, `/=`. También postfix `++` y `--`.

### Otros

`::` (namespace), `->` (flecha/tipo de retorno), `|` (unión de tipos),
`in` (pertenencia).

## Expresiones

Las expresiones admiten precedencia estándar:

```
a + b * c        // * antes que +
(a + b) * c      // paréntesis agrupan
x ? a : b        // condicional
f(x).campo[i]    // llamada, miembro, índice
```

### Literales de colección

```
[1, 2, 3]            // array (mutable)
(1, "dos", 3.0)      // tupla (inmutable)
{clave: 1, otro: 2}  // record / diccionario
```

## Terminación

Las sentencias terminan en `;`. Las declaraciones de tipos (class, enum,
interface, alias, structure, module, namespace) se cierran con `};`.

## Funciones flecha

```
var doble = (x: int) -> x * 2;
var suma = (a: int, b: int) -> { return a + b; };
```

La forma con cuerpo entre llaves admite múltiples sentencias.
