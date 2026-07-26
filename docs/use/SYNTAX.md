# Sintaxis de CLS

## Variables

```ccls
var x: Int = 10         # Variable con tipo
var name = "CLS"         # Variable con inferencia
const PI: Float = 3.14   # Constante
let y = 20               # let = var con inferencia
```

## Tipos

### Primitivos

| Tipo | Acrónimos | Descripción |
|------|-----------|-------------|
| `Int` / `Integer` | `int`, `i32`, `i64` | Número entero |
| `Float` | `float`, `f32`, `f64` | Número decimal |
| `String` | `str` | Cadena de texto |
| `Bool` / `Boolean` | `bool` | Verdadero o falso |
| `Char` / `Character` | `char` | Un carácter |
| `Any` | `any` | Cualquier tipo |
| `Void` | `void` | Nada (retorno de funciones) |
| `Null` | `null` | Valor nulo |

### Compuestos

```ccls
var arr: Int[] = [1, 2, 3]
var arr2: String[] = ["a", "b"]
var matrix: Int[][] = [[1, 2], [3, 4]]
var record: String{Int} = {"key": 123}
var fn: fun(Int): String = (x) -> String { toString(x) }
```

## Operadores

### Aritméticos
```
+  -  *  /  %  **
```

### Compuestos
```
+=  -=  *=  /=   ++  --
```

### Comparación
```
==  !=  <  >  <=  >=
```

### Lógicos
```
& (and)  | (or)  ! (not)
```

### Namespace
```
::   (math::abs)
```

### Arrow (tipo retorno)
```
->   (function f() -> Int)
```

## Strings

```ccls
var s = "hello"
var multiline = "line 1\nline 2"
var name = "CLS"
var greeting = "Hello, $name!"    # interpolación (pendiente)
var expr = "Result: ${1 + 2}"    # expresión interpolada (pendiente)
```

## Comentarios

```ccls
# Esto es un comentario de línea
```
