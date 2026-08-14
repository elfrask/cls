# Enums

Los enums son **literales con identidad**: a diferencia de los `alias` de
unión (solo tipado), cada variante tiene valor en runtime. Están pensados para
estados en sistemas embebidos: se compilan a un índice compacto (1-2 bytes,
`u8`/`u16`).

## Declaración

```clx
enum Color {
    Rojo,
    Verde,
    Azul,
};
```

- Variantes separadas por coma (la última también puede llevarla).
- Cada variante tiene identidad única: `def_name` + `index: u16` dentro del
  enum.

## Uso

```clx
function main(args: String[]) -> int {
    var c = Color.Rojo;
    print("valor:", c);            # Rojo
    print("== Rojo:", c == Color.Rojo);    # true (identidad)
    print("== Verde:", c == Color.Verde);  # false
    print("is Color:", c is Color);        # true
    return 0;
};
```

- Acceso namespaced: `Color.Rojo`.
- Comparación `==` por identidad (`def_name` + `index`), no por nombre.
- `is` por identidad del `def_name`.
- `print(c)` imprime el nombre de la variante (`Rojo`, `Verde`, ...).
- Runtime: `Value::EnumDef` (la definición, p. ej. `Color`) y `Value::Enum`
  (un valor: `{ def_name, variant, index }`).

## Iteración

```clx
for each v in (Color) {
    print(" -", v);          # Rojo, Verde, Azul
}

for each v and i in (Color) {
    print(" - [$i]", v);     # i = 0, 1, 2
}
```

Iterar un `Value::EnumDef` recorre sus variantes en orden de declaración.

## En módulos

Los enums son exportables y funcionan con `include` (se inyectan sin
namespacing — ver `modulos.md`):

```clx
# lib/colores.clsx
export enum Color {
    Rojo,
    Verde,
    Azul,
};
```

```clx
include "lib/colores";

function main(args: String[]) -> int {
    var c = Color.Azul;
    print("color:", c, "== Azul:", c == Color.Azul, "is:", c is Color);
    return 0;
};
```

Ejemplo completo: `examples/jit-examples/modules/src/` y
`examples/audit/features/10-enums.clsx`.

## En el type checker

- Declarar `enum Color` registra el tipo `Named("Color")`.
- `Color.Rojo` tipa como `Named("Color")` (el checker trackea los nombres de
  enums en `enums: HashSet<String>`).
- El pattern matching de payload (extraer datos de variantes) queda abierto
  como mejora futura.