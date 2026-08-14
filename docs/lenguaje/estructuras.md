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

Ejemplo: `examples/audit/features/11-structs.clsx`.