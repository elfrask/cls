# Control de flujo

Sintaxis verificada en `examples/audit/test-features/jit-test/units/` y
`examples/audit/test-features/`. Todas las formas listadas aquí las soporta el
JIT.

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

## while

```clx
var i = 0;
while (i < 3) {
    print("while:", i);
    i++;
}
```

Condición vacía = `true`.

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