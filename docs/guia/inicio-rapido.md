# Inicio rápido

Esta guía te lleva de cero a tu primer programa CLS.

## Hola mundo

Crea un archivo `main.clsx`:

```
function main(args: String[]) -> int {
    print("Hola, CLS!");
    return 0;
};
```

Ejecútalo:

```
clx run main.clsx
```

Salida:

```
Hola, CLS!
```

El punto de entrada es la función `main`, que recibe los argumentos de la línea
de comandos como un array de cadenas y devuelve un código de salida entero.

## Variables y tipos

```
var nombre: String = "CLS";
var edad: int = 30;
const PI = 3.14159;        // const infiere un literal type
var activo = true;
```

Los tipos básicos son `Int`, `Float`, `String`, `Bool`, `Char`, `Null`, `Void`
y `Any`. Las declaraciones usan `var`, `const` y `let` (alias de `var`).

## Funciones

```
function sumar(a: int, b: int) -> int {
    return a + b;
};

print(sumar(2, 3));   // 5
```

## Colecciones

```
var lista = [1, 2, 3];           // array (mutable)
var tupla = (1, "dos", 3.0);     // tupla (inmutable)
var dic = {clave: 1, otro: 2};   // record / diccionario

print(lista.length);     // 3
print(tupla[1]);         // "dos"
print(dic.clave);        // 1
```

Los métodos de tipos primitivos se resuelven por tipo (sin objetos):
`"hola".upper()` devuelve `"HOLA"`, `[1,2].push(3)` muta el array en el lugar.

## Control de flujo

```
for each x in (lista) {
    print(x);
}

var i = 0;
while (i < 5) {
    i = i + 1;
}

if (edad >= 18) {
    print("Mayor");
} else {
    print("Menor");
}
```

## Enums

```
enum Color {
    Rojo,
    Verde,
    Azul,
};

var c = Color.Verde;
print(c);                    // "Verde"
print(c == Color.Verde);     // true
print(c is Color);           // true

for each color in (Color) {
    print(color);
}
```

## Clases

```
class Persona {
    var nombre: String;

    function main(nombre: String) {
        me.nombre = nombre;
    }

    function saludar() -> String {
        return "Hola, " + me.nombre;
    }
};

var p = Persona("Ana");
print(p.saludar());   // "Hola, Ana"
```

## Módulos

Crea `lib.clsx`:

```
export function doble(x: int) -> int {
    return x * 2;
};
```

Y `main.clsx`:

```
import "lib" as lib;

function main(args: String[]) -> int {
    print(lib.doble(4));   // 8
    return 0;
};
```

## Verificar tipos

```
clx check --strict main.clsx
```

El modo estricto valida las anotaciones de tipo. Si no hay errores, muestra
"No se encontraron errores de tipo."

## Siguientes pasos

- `lenguaje/sintaxis.md` — el detalle completo de la sintaxis.
- `lenguaje/tipos.md` — el sistema de tipos (tuplas, uniones, alias, interfaces).
- `guia/cli.md` — todos los subcomandos.
- `guia/configuracion.md` — el manifiesto `cls.json`.
