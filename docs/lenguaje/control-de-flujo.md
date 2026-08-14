# Control de flujo

Sintaxis verificada en `features/07-control-flujo.clsx` y
`tests/all-features-jit2.clsx`. Todas las formas listadas aquí las soporta el
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

`when` evalúa la arquitectura en compilación (no en runtime) y es soportado por
el JIT:

```clx
var saludo = "generic";
when (arch: cls-arch) {
    saludo = "native";
}
```

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

El intérprete resuelve `return`/`break`/`continue` con señales de flujo
(`Flow`); el JIT las maneja en la emisión WASM. No se requiere `break` al
final de bloques en `switch` ni `;` tras `}`.