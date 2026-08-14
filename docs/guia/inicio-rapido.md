# Inicio rápido

`clx run` compila CLS a WASM y lo ejecuta con el JIT (wasmtime). Es el
intérprete objetivo; el tree-walker (`--ast-walker`) está deprecado.

## Hola mundo

```clx
function main(args: String[]) -> int {
    print("Hola, CLS!");
    return 0;
};
```

```ps
clx run main.clsx
```

El programa debe declarar `main(args: String[]) -> int`; el valor de retorno
es el código de salida.

## Crear un proyecto

```ps
clx new my-app
cd my-app
clx run
```

`clx new` genera:

```
my-app/
├── cls.json          # manifiesto (entry: "src/main.clsx")
├── .gitignore        # modules/, dist/, .cls-types
├── modules/          # dependencias instaladas
└── src/
    └── main.clsx     # function main(args: String[]) -> int
```

## Variables

```clx
var x: int = 42;      # anotada
var y = 3.14;         # inferida (Float)
const PI = 3.14159;   # constante
let z = 7;            # let es alias de var
var flag: bool = true;
var ch = 'a';         # Char
var nulo = null;
var s: String = "hola";
```

Acrónimos de tipos: `int`, `float`, `String`, `bool`.

## Funciones

```clx
function sumar(a: int, b: int) -> int {
    return a + b;
}

function main(args: String[]) -> int {
    var doble = (x: int) -> x * 2;   # arrow function
    print("suma:", sumar(3, 4));
    print("doble:", doble(21));
    return 0;
};
```

`print("etiqueta:", valor)` imprime la etiqueta seguida del valor
(p. ej. `int: 42`).

## Colecciones

```clx
var lista = [1, 2, 3];        # Array
var tupla = (1, "dos", 3.0);  # Tuple (inmutable)
var dic = {clave: 1};         # Record

function main(args: String[]) -> int {
    print(lista.length);      # 3  (getter de primitivo)
    print(tupla[1]);          # "dos"
    print(dic.clave);         # 1
    return 0;
};
```

## Control de flujo

```clx
function main(args: String[]) -> int {
    # if / elif / else
    var n = 7;
    if (n > 10) {
        print("grande");
    } elif (n > 5) {
        print("mediano");
    } else {
        print("pequeno");
    }
    # while
    var i = 0;
    while (i < 3) {
        print("while:", i);
        i++;
    }
    # loop (infinito, sale con break)
    var j = 0;
    loop {
        j++;
        if (j == 2) { break; }
    }
    # for clásico
    for (var k = 0; k < 3; k++) {
        print("for:", k);
    }
    # for each (con y sin índice)
    var arr = [5, 6, 7];
    for each v in (arr) {
        print("each:", v);
    }
    for each v and idx in (arr) {
        print("each[$idx]:", v);
    }
    # switch: 'case (N)' y 'case default'
    var c = 2;
    switch (c) {
        case (1) { print("uno"); }
        case (2) { print("dos"); }
        case default { print("otro"); }
    }
    # with
    var obj = {x: 10, y: 20};
    with o in (obj) {
        print("with:", o);
    }
    return 0;
};
```

## Enums

```clx
enum Color {
    Rojo,
    Verde,
    Azul,
};

function main(args: String[]) -> int {
    var c = Color.Verde;
    print(c == Color.Verde);  # true (identidad)
    print(c is Color);        # true
    for each v in (Color) {
        print(" -", v);
    }
    return 0;
};
```

## Clases

```clx
class Persona {
    var nombre: String;

    function main(nombre: String) {
        me.nombre = nombre;
    }

    function saludar() -> String {
        return "Hola, " + me.nombre;
    }
};

function main(args: String[]) -> int {
    var p = Persona("Ana");
    print(p.saludar());       # Hola, Ana
    return 0;
};
```

`me` es el equivalente a `this`. El constructor se declara como
`function main(...)` dentro de la clase.

## Módulos

`lib.clsx`:

```clx
export function doble(x: int) -> int {
    return x * 2;
};
```

`main.clsx`:

```clx
import "lib" as lib;

function main(args: String[]) -> int {
    print("doble(4):", lib.doble(4));
    return 0;
};
```

Los imports se resuelven relativos al archivo que importa, luego
`modules/` del proyecto y luego los globales `~/.cls/modules/`.

## Verificación de tipos

```ps
clx check --strict main.clsx
```

Sin errores imprime (en verde) `No se encontraron errores de tipo.` y sale
con código 0. Con `--strict`, las asignaciones incompatibles son error.