# Control de flujo

## if / elif / else

```
if (condición) {
    ...
} elif (otra) {
    ...
} else {
    ...
}
```

La condición debe ser booleana (o convertible a `Bool`). El verificador de tipos
advierte si no lo es.

## while

```
while (condición) {
    ...
}
```

## loop

Bucle infinito. Se sale con `break`.

```
loop {
    if (terminó) { break; }
}
```

## for (tradicional)

```
for (var i = 0; i < 10; i = i + 1) {
    print(i);
}
```

El inicializador puede omitir la palabra `var` cuando es una expresión de
asignación. La sentencia de actualización se ejecuta al final de cada iteración.

## for each

Itera sobre una colección (array, tupla, record, enum u objeto iterable):

```
for each item in (colección) {
    ...
}
```

Con índice (usa `and`):

```
for each item and i in (colección) {
    print(i, item);
}
```

## switch

```
switch (valor) {
    case (patrón1) {
        ...
    }
    case (patrón2) {
        ...
    }
    default {
        ...
    }
}
```

Los patrones se comparan por igualdad con el valor. Si ninguno coincide, se
ejecuta `default` (si existe).

## try / catch / finally

```
try {
    ...
} catch (e) {
    ...
}
```

- El error capturado queda disponible como `e` (una cadena con el mensaje).
- Se pueden usar varios bloques `catch`.
- Un `finally` (si existe) se ejecuta siempre.

## break / continue

- `break` sale del bucle o `switch` más cercano.
- `continue` salta a la siguiente iteración.

Funcionan en `while`, `loop`, `for`, `for each` y dentro de bloques anidados.

## return

```
function f() -> int {
    return 42;
};
```

Devuelve un valor de la función. En funciones sin retorno, `return` sin valor.

## with

```
with (expresión) as nombre {
    ...   // 'nombre' disponible en el bloque
}
```

Introduce un valor en el ámbito del bloque con un nombre local.

## Propagación de señales

El intérprete usa una señal de flujo interna (`Flow`) para `return`, `break` y
`continue`. Los bucles capturan y limpian la señal tras cada bloque, de modo que
no se "escapa" a bucles externos. Un `try/catch` restaura la profundidad de la
pila de llamadas al capturar un error.
