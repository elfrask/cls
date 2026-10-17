# Control de flujo

Sintaxis verificada en `examples/test/jit-test/units/` y
`examples/test/jit-test/`. Todas las formas listadas aquí las soporta el
JIT.

> **Los paréntesis de la condición son OBLIGATORIOS** en `if`, `elif`, `while`,
> `for`, `switch` y `when`: `if (condicion) { ... }`. A diferencia de Rust o
> Swift, CLS **no** acepta `if condicion { ... }` — es un error de sintaxis
> (`Esperaba símbolo '('`).

## if / elif / else

```clx
var n = 7;
if (n > 10) {
    print("grande");
} elif (n > 5) {
    print("mediano");
} else {
    print("pequeno");
}
```

La condición va **entre paréntesis**; la forma sin paréntesis no compila.

## while

```clx
var i = 0;
while (i < 3) {
    print("while:", i);
    i++;
}
```

Condición vacía = `true`. Los paréntesis son obligatorios (igual que en
`if`).

## loop (infinito)

```clx
var j = 0;
loop {
    print("loop:", j);
    j++;
    if (j == 2) { break; }
}
```

## for clásico

```clx
for (var k = 0; k < 3; k++) {
    print("for:", k);
}
```

`i++` y `++i` funcionan como actualización.

## for each

Sobre arrays (y tuplas), con o sin índice:

```clx
var arr = [5, 6, 7];
for each v in (arr) {
    print("each:", v);
}

for each v and idx in (arr) {
    print("each[$idx]:", v);
}
```

## switch

```clx
var c = 2;
switch (c) {
    case (1) { print("uno"); }
    case (2) { print("dos"); }
    case default { print("otro"); }
}
```

Nota: el caso lleva paréntesis `case (patrón)`, y el caso por defecto es
`case default` (sin paréntesis). Los bloques se terminan con `}` (sin `break`).

## match

`match` es una **expresión** (y también usable como sentencia) con **4 modos**
elegidos por la keyword tras el paréntesis. El cuerpo de arma es `-> expr`
(el valor implícito) o `-> { ... }` (bloque con `<-`). `default` es opcional; si
ninguna arma matchea y no hay
`default`, es un **error de runtime** ("ningún caso de `match` coincidió"). Un
`match` **vacío** (sin armas) es un **error de compilación**.

```clx
# Modo VALOR (`in`): igualdad de valor
var esPar: bool = match (n) in { (0, 2, 4, 6, 8) -> true, default -> false };

# Modo TIPO (`is`): por el tipo del valor (clases/enums por identidad runtime;
# builtins por el tipo estático)
var r = match (perro) is { Perro -> perro.ladra(), Gato -> perro.maulla(), default -> "?" };

# Modo TIPO multi-tipo (F5): `(A, B)` estrecha a la unión `A | B`; sobre la
# unión, `v.x` despacha por id de runtime y carga el campo en el offset propio
# de cada clase (ver tipos.md §Uniones).
var punto = match (v) is {
    (Vector2, Vector3) -> "(${v.x}, ${v.y})",
    default            -> "?",
};

# Modo CONDICIÓN (sin scrutinee, `if`): armas = condiciones Bool
var cmp = match() if { a > b -> ">", a < b -> "<", default -> "=" };

# Modo PATRÓN (sin keyword): variantes de enum (unit o con payload) / literales
var nombre = match (c <- Color.Verde) { Rojo -> "rojo", Verde, Azul -> "frio", default -> "?" };

# Modo PATRÓN con payload (F2): binding posicional
var out = match (v <- resp::ok("listo")) {
    ok(texto)     -> "OK ${texto}",
    err(cod, msg) -> "ERR ${cod} ${msg}",
    s1, s2        -> "unit",
    default       -> "?",
};
```

Reglas:

- **Armas**: `PATRONES -> expr`, separadas por `,`. Cada arma puede tener
  **varios patrones** con o sin paréntesis: `(0, 2, 4)`, `s1, s2`.
- **Binding**: `match (nombre <- expr) { … }` liga `nombre` al valor. Si el
  scrutinee es una variable (`match (x) { … }`) también sirve para el narrowing.
- **Narrowing**: en modo TIPO, si el scrutinee es un nombre, dentro de cada arma
  se estrecha al tipo del patrón (o a la unión si hay varios). En modo PATRÓN se
  estrecha al enum. Es lo que habilita `perro.ladra()` en el ejemplo.
- **Tipo del `match`**: lo fija la **primera arma**; las demás (y `default`)
  deben ser asignables a ese tipo (si hay anotación, se valida contra ella).
- **Statement vs expresión**: por posición. Como sentencia no devuelve valor
  (el cuerpo se dropea); como expresión lo devuelve.
- **Cuerpo de arma-bloque (F4)**: `PATRONES -> { ... }` con **arrowReturn `<-`**.
  El bloque es un *ternary-target*: entrega el valor del arma con `* <- expr`
  (y `match <- expr` salta directo al `match`). Ver [arrowReturn](#arrowreturn--).
- **Modo PATRÓN con payload (F2)**: el scrutinee debe ser un **enum**; los
  patrones van **desnudos** (`ok(texto)`, `err(cod, msg)`, `s1`). La variante se
  valida contra el esquema del enum (**variante inexistente / aridad incorrecta →
  error de compilación**); los bindings se ligan **por posición** al payload y
  `_` ignora un slot. A lo sumo **un patrón con bindings por arma**.
- **Destructuring-let (F2)**: `var resp::ok(result) = c;` (estático; ver
  `enums.md`).
- **Acceso a miembro sobre unión (F5)**: un arma con varios tipos `(A, B, …)`
  estrecha el scrutinee a la unión `A | B | …`. Sobre esa unión, `valor.<campo>`
  se resuelve en el typeck sólo si **todos** los tipos de la unión exponen el
  miembro (si no → error de compilación) y el emisor lo despacha por el **id de
  tipo en runtime** (`class_id` de clase en `[obj+8]`, `def_id` de struct en
  `[obj+0]`), cargando el campo en el **offset propio de cada miembro**. También
  aplica a `valor[i]` sobre una unión de arrays con el mismo ancho de elemento.
  Ver `tipos.md` §Uniones.

### Modo PATRÓN sobre una clase (F6)

Además de enums, el modo PATRÓN acepta una **clase que implemente el protocolo
de match**: `__match()` y `__unpack()` (ver `magics.md` §10).

- **`__match() -> E`** devuelve el **discriminante** (un valor de un enum `E`).
  El typeck usa `E` como **namespace de variantes**: los patrones desnudos se
  validan contra `E` (variante inexistente → error de compilación). En runtime el
  emisor llama `obj.__match()` **una vez** y compara contra el índice de la
  variante de cada arma.
- **`__unpack(tag: String) -> T | (T1, T2, …)`** devuelve el **payload
  computado**. Con bindings, el emisor llama `obj.__unpack("<variante>")` y liga
  **por posición**: si el arma tiene 1 binding, el resultado es un **valor
  único**; si tiene N, el resultado es una **tupla** y se leen sus slots.
- **Una clase sin `__match`** en modo PATRÓN es **error de compilación**. La
  **aridad** del patrón se valida best-effort contra el retorno anotado de
  `__unpack` (unión de tuplas → la aridad debe coincidir con algún miembro); si
  no es determinable, se valida en runtime (guard de aridad).

```clx
enum RespKind { ok, err };

class Resp {
    var kind: RespKind;
    var code: int;
    var body: String;
    function main(k: RespKind, c: int, b: String) { me.kind = k; me.code = c; me.body = b; }
    function __match() -> RespKind { return me.kind; }
    function __unpack(tag: String) -> (String) | (int, String) {
        if (tag == "ok") { return (me.body); }
        return (me.code, me.body);
    }
};

var out = match (v <- Resp(RespKind.ok, 0, "listo")) {
    ok(texto)     -> "ok ${texto}",
    err(cod, msg) -> "err ${cod} ${msg}",
    default       -> "?",
};
```

Los **otros modos** funcionan sobre clases sin cambios: TIPO (`is` → identidad de
clase + herencia), VALOR (`in` → `__equals` si está definido) y CONDICIÓN
(`if` → `__bool` si está definido).

> **Destructuring-let de clase (no implementado en F6).** El destructuring-let
> estático de enums (`var resp::ok(x) = c;`) no tiene hoy una forma análoga para
> clases: el payload **computado** de `__unpack` no se puede probar en
> compile-time, así que una variante de la sintaxis (`var Clase::variante(...) =
> obj;`) tendría que ser **runtime** y usar el enum `E` de `__match` como
> namespace. Se deja **fuera de F6** (el patrón `match` ya cubre la extracción);
> si se agrega, será con esa sintaxis (namespace = `E`) y validación en runtime.

## Familia ternario

Tres formas de elegir un valor según una condición. Solo la `?:` no usa `<-`.

### `?:` estilo JS (solo expresión)

```clx
var mayor = 10 > 5 ? true : false;
```

- Precedencia **la más baja** (por encima de la asignación) y **asociativa a
  derecha**: `a ? b : c ? d : e` == `a ? b : (c ? d : e)`.
- Es una **expresión pura**: **no** es target de `<-`. El ternario legacy
  `if (c) then (a) else (b)` sigue funcionando (ver abajo).

### `if` como expresión (bloques + `<-`)

```clx
var mayor = if (10 > 5) { * <- true } else { * <- false }
```

- Es una **expresión**: se parsea cuando en posición de expresión aparece
  `if (...) { ... } else { ... }`. **Requiere `else`** (si falta → error de
  compilación claro).
- Cada rama entrega su valor con **`* <- expr`** (arrowReturn, ver abajo); las
  ramas son *ternary-targets*. El tipo del `if`-expresión es el del `<-` de sus
  ramas.
- Distinción con el legacy: tras `if (cond)` viene `then` → ternario legacy;
  `{` → expresión `if`.

> **Gap conocido (F3)**: la `if`-expresión no admite `elif` (`if (a) { … } elif
> (b) { … } else { … }` → error de sintaxis). Para más de dos ramas, anidar
> `if`-expresiones o usar `match () if { … }`.

### Ternario legacy

```clx
var x = if (c) then (1) else (2);
```

Se conserva por compatibilidad; la forma recomendada hoy es `?:` o la
expresión `if`.

## arrowReturn (`<-`)

Devuelve un valor desde un bloque al **target** léxico indicado por las
estrellas (misma función; **no** hay cross-function):

```clx
enum R { a(int), b };

function calcular(iva: bool, monto: float) -> float {
    return match (R::a(0)) {
        a(n) -> {
            var base = if (iva) {
                * <- monto * 0.16        # -> el if-expresión (más cercano)
            } else {
                match <- monto           # -> el match (salta el if)
            }
            * <- base + monto            # -> el arma-bloque
        },
        b -> { * <- 0.0 }
    };
};
```

- **ternary-targets** (estrellas): la rama de un `if`-expresión o el cuerpo de
  un arma de `match` bloque. `*` = el más cercano; `**` = salta 1; `***` = salta
  2; ...
- **match-targets** (`match`): cada `match` envolvente. `match` = el más
  cercano; `match*` = salta 1; `match**` = salta 2; ...
- Atraviesa `if`/`while`/`for` intermedios (el `br` sale del loop/nivel al
  bloque-result del target). **Sin target** (no hay `if`-expresión ni arma de
  `match` bloque envolvente) → **error de compilación**.
- Un arma-bloque **debe** terminar con `<-` (si no, el valor del arma no
  existe): el error es de compilación.

## with

```clx
var obj = {x: 10, y: 20};
with o in (obj) {
    print("with:", o);
}
```

## break / continue / return

Válidos en bucles y funciones respectivamente:

```clx
for (var m = 0; m < 5; m++) {
    if (m == 1) { continue; }
    if (m == 4) { break; }
    print("bc:", m);
}
```

## when (compile-time)

`when` selecciona código por **entorno objetivo** (SO, arquitectura, ABI) en
**compilación** (no en runtime). El JIT solo emite la rama que matchea el
`Target` activo; las demás se descartan. La condición **lleva paréntesis**:

```clx
var saludo = "generic";
when (os: windows) {
    saludo = "native";
}
```

- Prefijos: `os:`, `arch:`, `abi:`, `platform:`, `target:` (tripla
  `arch-os-abi`).
- Combinadores `and` / `or` / `not` con paréntesis por condición:

```clx
when (os: linux) or (os: macos) {
    # unix
}
```

- `default { ... }` es la rama que siempre matchea.
- `when` funciona a **cualquier nivel de anidamiento** (no solo top-level) y
  típicamente envuelve bloques `extension` (FFI por plataforma — ver
  `extension-when.md`).

Referencia completa: [multi-entorno.md](multi-entorno.md).

## is (type guard / narrowing)

El operador `is` además de validar el tipo **estrecha la variable** dentro del
bloque: el typeck usa el tipo de la derecha para los usos posteriores.

```clx
if (app.tag is Callable) {
    app.tag({...app.props, children: app.children});  # aquí tag: Callable
}
```

- `x is Clase` → en el bloque `x` se trata como `Clase` (acceso a campos y
  métodos sin error de tipo).
- `x is Callable` → el chequeo runtime usa el tag-bit del handle (función
  simple vs closure); habilita invocar un tag de componente CMX.

Ver `oop.md` para `is` sobre clases y `tipos.md` para el tipo `Callable`.

## spreads de contenedores

El operador `...` (spread) expande un contenedor dentro de otro:

```clx
# record: mezcla (los props propios ganan sobre el spread)
var base = {a: 1, b: 2};
var ext = {...base, c: 3};          # {a: 1, b: 2, c: 3}

# array: expande en posición
var arr = [0, ...base_vals, 4];     # [0, 1, 2, 4]

# props CMX (JSX-style)
<App {...app.props} />
```

Los spreads se resuelven en compile-time/runtime del emisor
(`__intr_record_merge` para records/props, construcción con push para arrays).
No confundir con *rest params* (`function f(...nums)`) ni *spread en llamadas*
(`f(1, ...args)`), que **no están implementados** (el `call_indirect` exige
aridad estática).

## try / catch / finally

```clx
try {
    nivel1();
} catch (e) {
    print("catch:", e);
} finally {
    print("finally ejecutado");
}
```

El lanzamiento se hace con la intrinsic `throw(msg)`. En el JIT (wasmtime)
los errores llevan el caret exacto; con `CLS_JIT_RUNTIME=wasmi` no hay
soporte de excepciones (ver `runtime/jit.md`).

## Flujo interno

El JIT maneja `return`/`break`/`continue` en la **emisión WASM**: cada bloque
termina con ramas que cortan el flujo (`br`/`unreachable` tras un `return`), y
el emisor cierra las funciones con un default de retorno para flujo vivo sin
`return` (dead-flow). No se requiere `break` al final de bloques en `switch` ni
`;` tras `}`.