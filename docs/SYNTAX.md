# CLS 2.0 — Sintaxis del lenguaje

Basado en el rediseño parcial de v2.0. Extiende la sintaxis de v1.2 con CMX (JSX nativo)
y configuración modular.

---

## Configuración inline

```ccls
#!cls
#config(typecheck = true, typestrict = false)
```

---

## Variables y constantes

```ccls
var x: Int = 10              # Variable con tipo
const PI: Float = 3.1416     # Constante

# Tipos acrónimos
var a: int = 1               # Integer
var b: str = "hello"          # String
var c: float = 3.14           # Float
var d: bool = true            # Boolean
```

---

## Funciones

```ccls
# Función con tipado completo
function add(a: Int, b: Int) -> Int {
    return a + b
}

# Función void (sin retorno)
void log(msg: String) {
    print(msg)
}

# Función flecha anónima
var double = (x: Int) -> Int { x * 2 }

# Función flecha con tipado completo
var fn: fun(Int): String = (x: Int) -> String {
    return "Value: $x"
}

# Método de clase (usa me = this/self)
function saludar() -> String {
    return "Hola, $me.name"
}

# Modificadores
static function version() -> String { "1.0" }
async function fetch(url: String) -> String { ... }
export var CONFIG: Config = {...}
```

---

## Control de flujo

```ccls
# If / elif / else
if (cond) {
    ...
} elif (cond2) {
    ...
} else {
    ...
}

# Expresión condicional ternaria
var result = if (debug) then ("dev") else ("prod")

# While
while (cond) {
    ...
    break
    continue
}

# Loop infinito
loop {
    ...
    break
}

# For tradicional
for (i: Int = 0; i < 10; i++) {
    ...
}

# For each (iteración por elementos)
for each elem in (array) {
    print(elem)
}

for each elem and idx in (array) {
    print("$idx: $elem")
}

# Switch / case
switch (value) {
    case ("v1") { ... }
    case default { ... }
}

# Try / catch / finally
try {
    ...
} catch (e: Error) {
    ...
} finally {
    ...
}

# With (ámbito local)
with db in (openDb("localhost")) {
    db.query(...)
}

# Return
return value
```

---

## Clases, estructuras e interfaces

```ccls
# Clase con herencia
class MiClase(Base) {
    private var x: Int = 0
    export var name: String

    function main(name: String) {
        me.name = name
    }

    static function version() -> String {
        return "1.0"
    }
}

# Estructura (record con default values)
structure Config {
    host: String = "localhost",
    port: Int = 3000,
    debug: Bool = false
}

# Uso de estructuras
var cfg: Config = Config({debug: true})

# Interfaz (solo firmas)
interface Logger {
    fun log(level: String, msg: String): Void
}
```

---

## Módulos y namespaces

```ccls
module Utils {
    function parseJson(text: String) -> Any { ... }
}

# Namespace access con ::
var data = Utils::parseJson(json)
```

---

## Imports

```ccls
import "fs" as fs
from "http" import get, post as p
include "stdlib"
```

---

## CMX — JSX nativo

```ccls
# CMX genera estructuras de datos, no UI
# Módulos externos consumen esas estructuras

var app = (
    <App>
        Hello world
        <Dashboard />
        <Button name="test" click={async () -> {
            print("boton clickeado!")
        }} />
    </App>
)

# Regla de resolución de tags:
# <tag>    → minúscula → string tag "tag"
# <Tag>    → mayúscula → referencia a variable/función Tag

# Se traduce internamente a:
# createElement("App", null,
#     "Hello world",
#     createElement("Dashboard", null),
#     createElement(Button, { name: "test", click: handler })
# )
```

---

## Operadores

```ccls
# Aritméticos
a + b, a - b, a * b, a / b, a % b, a ** b

# Compuestos
a += 1, a *= 2
a++, a--

# Comparación
a == b, a != b, a < b, a > b, a <= b, a >= b

# Lógicos
a & b    # and
a | b    # or
!a       # not
a ? b    # in
a ^ b    # ** (pow)

# Namespace
ns::member

# Arrow (tipo retorno)
function f() -> Int
```

---

## Strings e interpolación

```ccls
# String básico
var s = "Hello"

# Interpolación
var msg = "Hola, $nombre"
var msg2 = "Tienes ${edad + 1} años"

# Tipos parametrizados
var arr: String[] = ["a", "b"]
var record: String{Int} = {"key": 123}
var matrix: Int[][] = [[1, 2], [3, 4]]
```

---

## Comentarios

```ccls
# Esto es un comentario de línea
```
