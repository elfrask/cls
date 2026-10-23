# Funciones

Sintaxis verificada en `features/08-funciones.clsx` y
`tests/all-features-jit2.clsx`. Todo lo documentado aquí lo soporta el JIT
salvo donde se indica.

## Declaración

```clx
function suma(a: int, b: int) -> int { return a + b; };
function saludar(nombre: String) { print("Hola", nombre); };
```

- Parámetros con anotación de tipo (`a: int`).
- Retorno opcional: sin `-> Tipo` la función es `void` (retorna sin valor).
- `;` final opcional después de `}`.

### Parámetros con valor por defecto

```clx
function suma(a: int, b: int = 5) -> int {
    return a + b;
};

print(suma(2));   # 7
```

## Recursión

Recursión normal (sin optimización de cola documentada):

```clx
function factorial(n: int) -> int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
};

function fib(n: int) -> int {
    if (n < 2) { return n; }
    return fib(n - 1) + fib(n - 2);
};
```

## Arrow functions

Tres formas: expresión, cuerpo con `return`, y sin argumentos:

```clx
var doble = (x: int) -> x * 2;
var multi = (a: int, b: int) -> { return a * b; };
var sinArgs = () -> 99;

print(doble(21));    # 42
print(multi(3, 5));
print(sinArgs());
```

## Closures

Capturan variables del entorno léxico:

```clx
var base = 10;
var closure = (x: int) -> x + base;
print(closure(5));          # 15
base = 100;
print(closure(5));          # 105 (ve el nuevo valor)
```

El JIT promueve las capturas al heap.

## Funciones como valor

Las funciones se pueden guardar y pasar:

```clx
print(type(suma));          # <function suma>
```

`Array.map(fn)` recibe una función como argumento (ver `datos.md`).

## main

`main(args: String[]) -> int` es el punto de entrada; el valor de retorno es
el exit code del proceso.

```clx
function main(args: String[]) -> int {
    return 0;
}
```

## Genéricos

```clx
function id<T>(x: T) -> T {
    return x;
};

var g: Int = id(5);
var h: String = id("hola");
var f: Float = id(3.5);
var b: Bool = id(true);
```

La inferencia cubre formas anidadas (`ObjType<T>` -> `{value:T}`, arrays,
records, tuplas). Un argumento incompatible con el parámetro es error de
compilación. Ver `tipos.md` para defaults `<T=Default>`, phantom `!T` y el
estado de los métodos de clases genéricas.

## Módulos

`export function` exporta la función al cargar el módulo:

```clx
export function suma(a: int, b: int) -> int { return a + b; };
```

Ver `sistema de módulos` en el contexto del proyecto.

## async / await

`async function`/`await` existen en el parser, pero **el JIT no compila
`await`** (error explícito del emisor) y ya no existe ningún intérprete que lo
ejecute (el tree-walker se eliminó en dev-2). No usarlos: error de compilación
en `clx run`. Ver `estado-runtime.md`.