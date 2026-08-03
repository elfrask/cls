# Sistema de tipos

Todo el sistema de tipos tiene efecto en el verificador (`clx check`), en el
modo estricto y en la futura compilación. No genera código en runtime.

## Tipos primitivos

`Int`, `Float`, `String`, `Bool`, `Char`, `Null`, `Void`, `Any`.

## Tuplas heterogéneas por posición

```
var par: (Int, String) = (1, "x");
par[0];              // Int (índice literal → slot exacto)
var i = 0;
par[i];              // Int | String (índice dinámico → unión de slots)
```

- Las tuplas son inmutables: `par[0] = 9` es error.
- `is_assignable_to` verifica posición a posición.

## Arrays

```
var lista: Int[] = [1, 2, 3];
```

- Homogéneos: el verificador infiere `Array<T>` del primer elemento.
- Mutable: `lista.push(4)` modifica el array en el lugar.

## Records / diccionarios tipados

```
alias Dict = Record<String, Int>;
var d: Dict = {a: 1};
```

- `Record<K, V>` es un diccionario clave → valor.
- El verificador infiere `Record<String, T>` desde un literal.
- Se escribe canónicamente `Record<K, V>` (la forma `K{V}` no se usa por
  ambigüedad con bloques).

## Uniones y literales (TypeScript-style)

```
alias Color = "red" | "green" | "blue";

var c: Color = "red";       // ok
var d: Color = "purple";    // error en estricto

const k = "constante";      // const infiere el literal "constante"
var v = "x";                // var infiere String (base), no el literal
```

- `Type::Literal` representa un valor concreto (`"red"`, `5`, `true`).
- `Type::Union` es la unión de varios tipos.
- Un literal es asignable a su tipo base; un valor solo es asignable a un
  literal si coincide exactamente.

## Alias de tipos

```
alias Vec3 = (Int, Int, Int);        // tupla
alias FnInt = (Int) -> Int;          // función
alias Color = "red" | "green";       // unión de literales
alias Dict = Record<String, Int>;    // diccionario
```

- La palabra clave es `alias` (evita la colisión con el intrinsic `type(val)`).
- Solo tiene efecto en el verificador (compile-time).

## Interfaces y shapes

```
interface Hello<T=Int> {
    num: T,
    greet(name: String): String,
};
```

- Las interfaces declaran campos (`nombre: tipo`) y métodos
  (`nombre(params): tipo`). Los shapes (claves fijas) se declaran solo en
  `interface`.
- Tienen efecto solo en el verificador.

## Extracción de tipos hijos

```
var n: Hello["num"] = 1;             // Int (el default de T)
var s: Hello<String>["num"] = "x";   // String (T sustituido)
var t: (Int, String)[1];             // String (slot 1 de la tupla)
```

- `T["campo"]` extrae el tipo del campo (o del método, que es un tipo función).
- `T[índice]` extrae por posición (tuplas, arrays, interfaces).
- Aplica los argumentos genéricos o los defaults.

## Genéricos

```
function id<T>(x: T) -> T { return x; };
class Caja<T> {
    var contenido: T;
    function main(contenido: T) { me.contenido = contenido; }
    function obtener() -> T { return me.contenido; }
};
interface Marcador<T=Int> { dato: T };
```

- Funciones: el verificador infiere `T` desde los argumentos al llamar.
- Clases: se declaran y verifican con placeholders; la instanciación completa
  (`Caja<String>` con verificación de miembros) está planeada.
- Interfaces: los parámetros se sustituyen en la extracción de tipos.

## Phantom `!T`

```
interface Marcador<T> {
    real: T,
    fantasma: !T,
};
```

- `!T` marca un parámetro que NO participa en el tipo del miembro: no se
  sustituye ni se unifica.
- Sirve para parámetros "fantasma" que solo existen a nivel de identidad del
  tipo.

## Enums (resumen)

Los enums son literales con identidad; ver `lenguaje/enums.md`.

## Reglas de asignación

La asignación es válida si el tipo de la expresión es asignable al tipo
declarado:

- `Any` es asignable a cualquier cosa y viceversa.
- Tipos idénticos son asignables.
- `Int` es asignable a `Float`.
- Un literal es asignable a su tipo base o a un literal idéntico.
- Una unión es asignable si alguno de sus miembros lo es.
- Las tuplas se comparan posición a posición.
