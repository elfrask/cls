# Sistema de tipos (compile-time)

El sistema de tipos tiene efecto en el typechecker (`clx check`), en el modo
estricto (`clx check --strict`) y en la compilación a WASM (el JIT corre
siempre con typecheck estricto). Fuentes: `cls-core/src/middleware/types.rs` y
`typeck.rs`; ejemplos en `features/17-genericos.clsx`, `features/18-shapes.clsx`
y `tests/test-types.clsx`.

## Tipos base

| Tipo | Notación | Notas |
|---|---|---|
| Entero | `Int` / `int` | `i64` |
| Flotante | `Float` / `float` | `f64` |
| String | `String` / `str` | |
| Bool | `Bool` | |
| Char | `Char` | sin literal propio (ver `sintaxis.md`) |
| Any | `Any` | universal; el JIT exige anotación explícita |
| Unknown | `Unknown` | |
| Null | `Null` | |
| Void | `Void` | sin valor (`void function`) |
| Empty | `Empty` | |

Acrónimos numéricos: `i32`, `i64`, `i16`, `i8`, `f32`, `f64`.

Conversión implícita: `Int -> Float`.

## Arrays

`T[]` es un array homogéneo. Los literales heterogéneos se promueven a
`Float` (subset JIT). Covarianza de elementos.

```clx
var a: Int[] = [1, 2, 3];
```

## Tuplas heterogéneas por posición

```clx
var a: (Int, String) = (1, "x");
a[0];   # Int (índice literal -> slot exacto)
a[i];   # Int | String (índice dinámico -> unión de slots)
```

## Records tipados

```clx
alias Dict = Record<String, Int>;
var d: Dict = {a: 1};
```

Los literales `{clave: valor}` infieren `Record<String, T>`.

## Shapes (interfaces)

```clx
interface Persona {
    nombre: String,
    edad: int,
};

var p: Persona = { nombre: "Ana", edad: 30 };
print(p.nombre);
```

Un shape es assignable a `Record<String, T>` si sus claves son `String`.

## Uniones y literal types (estilo TypeScript)

```clx
alias Color = "red" | "green" | "blue";
var c: Color = "red";    # ok
var d: Color = "purple"; # ERROR en estricto
```

Inferencia de literales: `const k = "constante";` infiere el literal
`"constante"`; `var v = "x";` infiere `String` (tipo base).

## Alias

```clx
alias Name = tipo;
alias Vec3 = (Int, Int, Int);
alias FnInt = (Int) -> Int;
```

Solo compile-time. Cubren tuplas, uniones, funciones y records.

## Interfaces genéricas y extracción de tipos hijos

```clx
interface Hello<T=Int> {
    num: T,
    greet(name: String): String,
};

var n: Hello["num"] = 1;            # Int (default)
var s: Hello<String>["num"] = "hola";  # String
var t: (Int, String)[1];            # String
```

La extracción `T["clave"]` / `T[índice]` aplica los genéricos dados (o sus
defaults) y resuelve campos de interfaces (también métodos -> tipo función),
tuplas, arrays, records y uniones.

## Genéricos

```clx
function id<T>(x: T) -> T {
    return x;
};

class Caja<T> {
    var contenido: T;
    function main(contenido: T) {
        me.contenido = contenido;
    }
    function obtener() -> T {
        return me.contenido;
    }
};

var g: Int = id(5);
var caja = Caja(42);
```

- `function`/`class`/`interface` aceptan parámetros de tipo con default:
  `<T=Int>`.
- `!T` (phantom): el parámetro no se sustituye ni se unifica.

```clx
interface Marcador<T> {
    real: T,
    fantasma: !T,
};
```

## Intersección de shapes

`A & B` fusiona ambos shapes; si un campo conflictúa en tipo, es error.

## Typechecking

- `clx check` - chequea un nivel (un solo archivo).
- `clx check --strict` - asignaciones incompatibles son error;
  `no_implicit_any` aborta variables sin tipo.
- El JIT (`clx run`) compila siempre con typeck estricto (`strict`,
  `no_implicit_any`, `null_safety`); los errores de tipo abortan antes de
  emitir el binario.
