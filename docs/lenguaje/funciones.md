# Funciones

## Declaración

```
function nombre(parámetro: Tipo, otro: Tipo) -> TipoDeRetorno {
    return valor;
};
```

- `function` para funciones con retorno.
- `function` sin `-> Tipo` (o `void nombre(...)`) para procedimientos.
- La función se cierra con `};`.

Parámetros con valores por defecto:

```
function saludar(nombre: String, saludo: String = "Hola") -> String {
    return saludo + ", " + nombre;
};
```

## Retorno

`return` devuelve un valor y detiene la función. En procedimientos, `return`
sin valor termina la ejecución.

## Llamadas

```
var r = sumar(2, 3);
```

Los argumentos se evalúan en orden y se pasan por valor (los valores se clonan;
los objetos y arrays comparten referencia en el intérprete).

## Funciones flecha

```
var doble = (x: int) -> x * 2;
var aplicar = (a: int, b: int) -> { return a * b; };
```

- Con cuerpo de expresión: `(x) -> expresión`.
- Con cuerpo de bloque: `(x) -> { ... }` (admite múltiples sentencias).

Las funciones flecha son valores asignables y capturan el entorno léxico
(closures).

## Genéricos

```
function id<T>(x: T) -> T {
    return x;
};
```

- `T` es un parámetro de tipo (compile-time).
- Al llamar `id(5)`, el verificador infiere `T = Int` y sustituye en el retorno:
  `var n: Int = id(5);` es válido.

## Funciones como valores

Una función declarada se puede asignar:

```
var f = sumar;
print(f(1, 2));
```

## Async / Await

```
async function descargar() -> String {
    var res = await operacion();
    return res;
};
```

- `async function` crea una corrutina: al llamarla devuelve una `Promise` y el
  cuerpo no se ejecuta hasta que se "consume".
- `await` espera el resultado de una expresión que devuelve `Promise`.
- El módulo `async` ofrece utilidades (`delay`, `all`, `race`).

## Visibilidad y export

Una función puede llevar modificadores:

```
export function publica() -> int { return 1; };
```

- `export` la hace disponible en módulos importados.
- `public`/`private`/`protected`/`static` se usan como miembros de clase.

## Recursión

Las funciones pueden llamarse a sí mismas:

```
function factorial(n: int) -> int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
};
```

## Firma de tipos de función

Como anotación, un tipo de función se escribe `(Int) -> Int` (o
`fun(Int) -> Int`):

```
alias Operacion = (Int, Int) -> Int;
var suma: Operacion = (a: int, b: int) -> a + b;
```
