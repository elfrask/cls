# Métodos de tipos primitivos (sin boxing)

Los tipos primitivos (`String`, `Int`, `Float`, `Bool`, `Char`, `Array`,
`Tuple`, `Record`) tienen métodos, pero **no se envuelven en objetos**. En lugar
de almacenar métodos en cada valor, se resuelven por **tablas de despacho
estáticas** en `cls-runtime/src/stdlib/primitive.rs`:

```
tipo primitivo → { nombre → Method(NativeFn) | Getter(NativeFn) }
```

## Cómo funciona

- El valor primitivo permanece plano (un dato del enum `Value`).
- El *receiver* viaja como el primer argumento (`args[0]`) de la función nativa.
- Un **Getter** (`"hola".length`) se evalúa en el acceso a miembro directo.
- Un **Method** (`"hola".upper()`) devuelve un `FunValue` enlazado al receiver,
  con nombre interno `__method__.nombre` (para no colisionar con intrinsics
  interceptados por nombre).

## Por qué es compatible con binario nativo

Como el tipo de un primitivo se conoce en compile-time, un compilador nativo
puede **monomorfizar**: devolver la dirección directa del método (por ejemplo,
`string_upper`) sin buscar en una tabla en runtime ni crear un wrapper. Esto
encaja con el modelo de `docs/future/native/` (tipos planos `i64`/`f64`/`ptr`).

## Semántica

- **Strings y números son inmutables**: los métodos de transformación devuelven
  un valor nuevo. El usuario asigna explícitamente: `s = s.upper();`.
- **Arrays son mutables**: los mutadores (`push`, `pop`, `shift`, `unshift`,
  `reverse`) reciben el array, lo modifican y devuelven el array mutado.
  `evaluate_call` hace *write-back* automático a la variable cuando el receiver
  es un identificador y el resultado es un `Array`.

## Catálogo

### String

`upper()`, `lower()`, `trim()`, `contains(s)`, `startsWith(s)`, `endsWith(s)`,
`isEmpty()`, `toString()`, getter `length`.

### Array

`push(x)`, `pop()`, `shift()`, `unshift(x)`, `indexOf(x)`, `includes(x)`,
`join(sep)`, `reverse()`, `toString()`, getter `length`.

### Tuple (inmutable)

`join(sep)`, `toString()`, getter `length`.

### Record

`keys()`, `values()`, `has(k)`, `toString()`, getters `length`, `size`.

### Int / Float

`toString()`, `abs()`.

### Bool / Char

`toString()`.

## Ejemplos

```
"Hola Mundo".upper();        // "HOLA MUNDO"
"hola".length;               // 4
[1, 2, 3].push(4);           // muta el array; [1,2,3,4]
(1, 2, 3).join("+");         // "1+2+3"
{a: 1}.has("a");             // true
(-7).abs();                  // 7
```

## Cómo agregar un método

Para añadir un método a un tipo, agrega una entrada en la tabla correspondiente
de `cls-runtime/src/stdlib/primitive.rs`:

```
t.insert("miMetodo", method(|a| { ... }));
```

El receiver está en `args[0]`. Usa los helpers `expect_string`, `expect_array`,
etc., para validar el tipo.
