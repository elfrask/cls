# Estructuras (`structure`)

Las estructuras son **datos planos** sin métodos ni herencia. Son la
alternativa ligera a las clases para agrupar campos.

## Declaración

```clx
structure Punto {
    x: int,
    y: int,
};
```

- Campos separados por coma con sintaxis `nombre: Tipo` (sin `var`).
- Sin métodos, sin herencia, sin genéricos (por diseño).

## Instanciación y acceso

```clx
function main(args: String[]) -> int {
    var p = Punto(3, 4);       # instanciación posicional (orden de campos)
    print("p.x:", p.x, "p.y:", p.y);   # p.x: 3 p.y: 4
    var p2 = Punto(1, 1);
    print("p2:", p2);          # Punto { x: 1, y: 1 }
    print("tipo:", type(p));   # Struct
    print("is Punto:", p is Punto);    # true
    return 0;
};
```

- La instanciación usa la sintaxis de llaamda `Punto(x, y)` con los valores en
  el orden de declaración de los campos.
- Acceso a campos con `p.campo`.
- `print(p)` produce `Punto { x: 3, y: 4 }`.
- `p is Punto` compara el `def_name`.

## Layout en memoria (ABI C)

Un `structure` se aloca en la memoria lineal con un **header CLS de 16 bytes**
(`[def_id:i64][n_campos:i64]`, usado por `is`/`print`) y a continuación la
**data con layout ABI C**: cada campo ocupa su tamaño natural y se alinea a él
(como el struct C equivalente).

| Tipo del campo | Bytes en memoria | Tipo C equivalente |
|---|---|---|
| `i8` | 1 | `char` / `int8_t` |
| `i16` | 2 | `short` |
| `i32` | 4 | `int` |
| `i64` | 8 | `long long` / `int64_t` |
| `f32` / `f64` | 4 / 8 | `float` / `double` |
| `int` / `float` (CLS) | 8 | `int64_t` / `double` |
| `bool` / `char` (CLS) | 4 | (convención FFI: `int`) |
| punteros / objetos / strings | 8 | puntero |
| `T[n]` | `n * sizeof(T)` | array inline (`char sin_zero[8]`) |
| `CInt` / `CShort` / `CChar` / `CDouble` / `CString` / `CPtr` | 1/2/4/8 | el tipo C homónimo |

- **`T[n]`** es un **array inline**: ocupa espacio en el layout pero no tiene
  acceso escalar (`p.buf` es error de compilación). No consume argumento del
  constructor — se usa para rellenar el struct C (`i8[257]` de `szDescription`).
- La alineación es natural: un `i64` tras un `i16` se salta los bytes de padding
  necesarios, igual que haría C.
- El offset C 0 coincide con el byte **+16** de la estructura CLS (tras el
  header). Es lo que hace que pasar el struct por FFI sea binario-compatible
  (ver `extension.md`).

```clx
# sockaddr_in de WinSock/glibc: 16 bytes, el mismo layout que el struct C.
structure sockaddr_in {
    sin_family: i16,      # offset 0
    sin_port: i16,        # offset 2
    sin_addr: i32,        # offset 4
    sin_zero: i8[8],      # offset 8
};
```

```clx
print(sockaddr_in(2, 9999, 0).sin_port);   # 9999 (lectura de 2 bytes)
```

> Nota: escribir un campo con un ancho C (`p.sin_port = 8080` donde el campo es
> `i16`) requiere un valor del tipo anotado — los literales `Int` no se asignan a
> `i8`/`i16`/`i32` (no hay narrowing implícito, ver
> `decisiones/001-no-implicit-coercion.md`). Se construye por el constructor
> posicional: `sockaddr_in(2, 8080, 0)`.

## En compilación nativa (futuro)

El backend AOT nativo (fuera del alcance actual: hoy se compila a WASM) está
planificado para compilar estructuras a memoria tipo C (layout plano de
campos), con campos de tipo complejo como punteros. No hay genéricos ni
inferencia en estructuras (decisión de diseño).

## Estructuras nativas (FFI)

También se pueden declarar dentro de un bloque `extension` para describir
layouts C de librerías del sistema (ver `extension.md`):

```clx
extension "msvcrt.dll" as C {
    structure Punto { x: int, y: int };
};
```

Ejemplo: `examples/test/jit-test/units/11-structs.clsx`.