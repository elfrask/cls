# Métodos de tipos primitivos

Los primitivos (`String`, `Int`, `Float`, `Bool`, `Char`, `Array`, `Tuple`,
`Record`) **no se envuelven en objetos**. Sus métodos se resuelven por
**dispatch tables estáticas** (`cls-runtime/src/stdlib/primitive.rs`): por tipo
primitivo hay una tabla `nombre → método`, y el receiver viaja como `args[0]`
(plano, sin boxing).

Esto es compatible con compilación nativa/WASM: el tipo del receiver se conoce
en compile-time, por lo que el compilador puede llamar la dirección directa del
método (monomorfización).

## Getters vs métodos

- **Getter** (`@kind getter`): propiedad computada, se lee **sin paréntesis** —
  `"hola".length`.
- **Método**: se invoca **con paréntesis** — `"hola".upper()`.

Las firmas canónicas viven en `cls-runtime/clsi/types.clsi`.

## `String` (inmutable)

Los métodos de transformación devuelven un string **nuevo** (asignar:
`s = s.upper()`).

```clx
"hola".upper();           # "HOLA"
"  x  ".trim();           # "x"
"hola".length;            # 4
```

| Miembro | Tipo | Firma | Descripción |
|---|---|---|---|
| `upper` | método | `upper() -> String` | Mayúsculas. |
| `lower` | método | `lower() -> String` | Minúsculas. |
| `trim` | método | `trim() -> String` | Recorta espacios en los extremos. |
| `contains` | método | `contains(needle: String) -> bool` | ¿Contiene el substring? |
| `startsWith` | método | `startsWith(prefix: String) -> bool` | ¿Empieza con el prefijo? |
| `endsWith` | método | `endsWith(suffix: String) -> bool` | ¿Termina con el sufijo? |
| `isEmpty` | método | `isEmpty() -> bool` | ¿Está vacío? |
| `toString` | método | `toString() -> String` | El string mismo. |
| `length` | **getter** | `length() -> int` | Longitud: **bytes en el JIT**, caracteres en el walker. |

## `Int` y `Float` (inmutables)

| Tipo | Miembro | Tipo | Firma | Descripción |
|---|---|---|---|---|
| `Int` | `toString` | método | `toString() -> String` | Representación decimal. |
| `Int` | `abs` | método | `abs() -> int` | Valor absoluto. |
| `Float` | `toString` | método | `toString() -> String` | Representación decimal. |
| `Float` | `abs` | método | `abs() -> float` | Valor absoluto. |

## `Bool` y `Char` (inmutables)

| Tipo | Miembro | Descripción |
|---|---|---|
| `Bool` | `toString() -> String` | `"true"` / `"false"`. |
| `Char` | `toString() -> String` | El carácter como string. |

## `Array` (mutable)

Los **mutadores** (`push`, `pop`, `shift`, `unshift`, `reverse`) modifican el
array in-place, devuelven el array mutado y `evaluate_call` hace **write-back
automático**: `arr.push(4)` equivale a `arr = arr.push(4)`.

```clx
var arr = [1, 2, 3];
arr.push(4);       # arr ahora es [1, 2, 3, 4]
arr.pop();         # arr ahora es [1, 2, 3]
arr.length;        # 3
```

| Miembro | Tipo | Firma | Descripción |
|---|---|---|---|
| `length` | **getter** | `length() -> int` | Número de elementos. |
| `push` | método (mutador) | `push(value: Any) -> Array` | Agrega al final. |
| `pop` | método (mutador) | `pop() -> Array` | Elimina el último. |
| `shift` | método (mutador) | `shift() -> Array` | Elimina el primero. |
| `unshift` | método (mutador) | `unshift(value: Any) -> Array` | Agrega al inicio. |
| `reverse` | método (mutador) | `reverse() -> Array` | Invierte el orden. |
| `indexOf` | método | `indexOf(value: Any) -> int` | Índice de la primera ocurrencia; `-1` si no está. |
| `includes` | método | `includes(value: Any) -> bool` | ¿Contiene el valor? |
| `join` | método | `join(separator: String) -> String` | Une los elementos; separador default `","`. |
| `toString` | método | `toString() -> Array` | Concatenación con separador `,`. |

## `Tuple` (inmutable)

Sin mutadores y sin asignación por índice (`t[0] = 99` → error). Solo lectura
por índice y `for each`.

| Miembro | Tipo | Firma | Descripción |
|---|---|---|---|
| `length` | **getter** | `length() -> int` | Número de elementos. |
| `join` | método | `join(separator: String) -> String` | Separa con el separador (default `","`). |
| `toString` | método | `toString() -> Tuple` | Concatenación con separador `,`. |

## `Record`

| Miembro | Tipo | Firma | Descripción |
|---|---|---|---|
| `length` | **getter** | `length() -> int` | Número de entradas. |
| `size` | **getter** | `size() -> int` | Número de entradas (igual a `length`). |
| `keys` | método | `keys() -> Array` | Claves **ordenadas**. |
| `values` | método | `values() -> Array` | Valores, ordenados por clave. |
| `has` | método | `has(key: String) -> bool` | ¿Existe la clave? |
| `toString` | método | `toString() -> Record` | Representación de pares clave/valor. |

> **Las claves propias tienen prioridad sobre los métodos**: `r.edad` accede a
> la clave `edad`; sobre un record sin clave `length`, `r.length` es el método
> (getter, sin paréntesis).