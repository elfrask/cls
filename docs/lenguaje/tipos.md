# Sistema de tipos (compile-time)

El sistema de tipos tiene efecto en el typechecker (`clx check`), en el modo
estricto (`clx check --strict`) y en la compilación a WASM (el JIT corre
siempre con typecheck estricto). Fuentes: `cls-core/src/middleware/types.rs` y
`typeck/`; ejemplos en `features/17-genericos.clsx`, `features/18-shapes.clsx`
y `tests/test-types.clsx`.

## Tipos base

| Tipo | Notación | Notas |
|---|---|---|
| Entero | `Int` / `int` | `i64` |
| Flotante | `Float` / `float` | `f64` |
| String | `String` / `str` | |
| Bool | `Bool` | |
| Char | `Char` | sin literal propio (ver `sintaxis.md`) |
| Unknown | `Unknown` | |
| Value | `Value` | valor CLS dinámico pero **tipado** (primitivo, array o JSON). Aceptado en params/retornos del JIT (i64); sin tag de primitivo, `match is` sobre `Value` es error. |
| JSON | `JSON` | documento JSON dinámico (objeto/array) |
| Cmx | `Cmx` | nodo CMX (tag/props/children) |
| Null | `Null` | |
| Void | `Void` | sin valor (`void function`) |
| Empty | `Empty` | |

Acrónimos numéricos: `i32`, `i64`, `i16`, `i8`, `f32`, `f64`.

> **`Any` fue eliminado por completo** (`Type::Any` ya no existe; `Value` ocupa
> su lugar como top universal del sistema). Como anotación de usuario (CLS es
> totalmente estático) usarlo es error. Para valores dinámicos tipados se usan
> `Value` (cualquier valor CLS), `JSON` (documentos/objetos) y `Cmx` (nodos CMX).
> Estos siguen siendo válidos: el tag runtime viaja con el valor y el emisor los
> trata como contenedores polimórficos concretos.

Conversión implícita: `Int -> Float` (ver [decisión 001](../decisiones/001-no-implicit-coercion.md)). Es la única coerción permitida en CLS 2.0; todo otro mismatch de tipos es error en `clx check`.

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

> **GAP runtime (JIT):** el typeck infiere `Int | String`, pero el JIT guarda los
> slots sin *tag*; leer con índice dinámico **heterogéneo** no recupera el tipo en
> runtime (requeriría boxing, prohibido). El índice literal y el dinámico
> homogéneo sí funcionan. Ver `examples/lang-features/findings/gaps.txt` (GAP 06).

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

### Herencia de interfaces (`extends`)

Una interface puede heredar campos y métodos de una o varias interfaces:

```clx
interface Base {
    id: String,
};

interface Labeled<T> {
    label: T,
};

interface Card extends Base, Labeled<String> {
    title: String,
};

var c: Card = { id: "1", label: "Hola", title: "Inicio" };
print(c.label);   # String (heredado y sustituido)
```

- `extends A, B<T>` acepta varias bases y argumentos genéricos.
- Los miembros heredados se fusionan; si el hijo redefine un campo, su tipo
  reemplaza al del padre.
- Los ciclos de herencia se ignoran (corte de recursión) y una base inexistente
  produce error en el typeck.
- Igual que el resto de interfaces, no genera código en runtime.

## Uniones y literal types (estilo TypeScript)

```clx
alias Color = "red" | "green" | "blue";
var c: Color = "red";    # ok
var d: Color = "purple"; # ERROR en estricto
```

Inferencia de literales: `const k = "constante";` infiere el literal
`"constante"`; `var v = "x";` infiere `String` (tipo base).

### Acceso a miembro sobre una unión (F5)

Una unión de tipos (por `alias` o por el estrechamiento / narrowing del `match`
multi-tipo, ver `control-de-flujo.md`) permite acceder a un miembro **siempre que
todos sus tipos lo expongan**. El resultado es el tipo común; si los tipos de ese
miembro difieren, el resultado es la unión de esos tipos.

```clx
class Vector2 { var x: float; var y: float; };
class Vector3 { var x: float; var y: float; var z: float; };
alias Vec = Vector2 | Vector3;

function describe(v: Vec) -> String {
    # narrowing a `Vector2 | Vector3` + `v.x` sobre la unión
    return match (v) is {
        (Vector2, Vector3) -> "(${v.x}, ${v.y})",
        default            -> "?",
    };
}
```

Si **algún** tipo de la unión no tiene el miembro, el compilador lo rechaza:

```clx
class A { var x: int; };
class B { var y: int; };
alias AB = A | B;
var v: AB = A();
v.x;   # ERROR: la unión no tiene el miembro `x` en todos sus tipos (falta en `B`)
```

En runtime el emisor despacha por el **id de tipo** del valor (clases: `class_id`
en `[obj+8]`; structs: `def_id` en `[obj+0]`) y carga el campo en el **offset
propio de cada miembro** (los layouts pueden diferir), sin boxing. El índice
`valor[i]` sobre una unión de arrays con el mismo ancho de elemento también está
soportado.

**Uniones de primitivos (tag runtime)**: una unión no literal de primitivos
(`int | String`, `Boolean | float`) se guarda como puntero a `[tag][valor]`
cuando cruza una variable, parámetro, retorno, campo o elemento de array. Esto
permite discriminarla de verdad:

```clx
alias M = String | int | Boolean;

function tipo(v: M) -> String {
    return match (v) is {
        String  -> "string",
        int     -> "int",
        Boolean -> "bool",
        default -> "?",
    };
}
```

`print(v)`, `str(v)`, `v.toString()` e interpolación usan el tag real. Las
uniones de **literales** (`"red" | "green"`) no se boxean: se discriminan
comparando el valor. Las conversiones numéricas `int(valor)` / `float(valor)`
sobre la unión siguen rechazadas en compile-time; hacelas dentro del brazo ya
estrechado. El acceso a miembro/método común sobre una unión de primitivos
(`.abs()` fuera de un brazo) sigue sin estar soportado.

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

## Callable

`Callable` es el tipo de los **valores invocables**: función simple, closure u
objeto con `__call`. En runtime se identifica por el tag-bit del handle
(par = función simple, impar = closure).

- Toda función es asignable a `Callable` (`Fun(..) → Callable`).
- `Callable` es asignable a `Value` (un callable es un valor).
- `Value`/`Cmx` NO son asignables a `Callable` en compile-time (son dinámicos —
  el `is Callable` resuelve en runtime).

```clx
var f = suma;                    # f: Fun(..)
if (f is Callable) {             # narrowing: f es Callable dentro del bloque
    f(2, 3);
}
```

`is Callable` chequea en runtime que el valor sea un handle de función
(`v != 0 && v>>32 == 0`). Ver `docs/lenguaje/cmx.md` (invocar componentes).

## Typechecking

- `clx check` - chequea un nivel (un solo archivo).
- `clx check --strict` - asignaciones incompatibles son error;
  `no_implicit_any` aborta variables sin tipo.
- El JIT (`clx run`) compila siempre con typeck estricto (`strict`,
  `no_implicit_any`, `null_safety`); los errores de tipo abortan antes de
  emitir el binario.
