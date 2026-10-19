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
namespacing - ver `modulos.md`):

```clx
# lib/colores.clsx
export enum Color {
    Rojo,
    Verde,
    Azul,
};
```

```clx
from "lib/colores" import *;

function main(args: String[]) -> int {
    var c = Color.Azul;
    print("color:", c, "== Azul:", c == Color.Azul, "is:", c is Color);
    return 0;
};
```

Ejemplo completo: `examples/test/jit-examples/modules/src/` y
`examples/test/jit-test/units/10-enums.clsx`.

## En el type checker

- Declarar `enum Color` registra el tipo `Named("Color")`.
- `Color.Rojo` tipa como `Named("Color")` (el checker trackea los nombres de
  enums en `enums: HashSet<String>` y el **esquema** de variantes/params en
  `enum_defs`).
- `X::v(args)` se valida estáticamente (variante, aridad y tipos) y tipa
  `Named(X)`.

## Variantes con payload (F2)

Las variantes pueden llevar **payload** (una tupla de tipos). Se mezclan
libremente con las variantes unit:

```clx
enum resp { ok(String), err(int, String), s1, s2 };
```

- **Construcción cualificada**: `resp::ok("Hola")`, `resp::err(404, "no")`.
  `ok(...)` desnudo **no** construye (solo vale como patrón). Las variantes unit
  se acceden con `resp::s1` o `resp.s1`.
- **Patrón declarativo** en `match` modo patrón: bindings **posicionales**
  (`err(cod, msg)` liga `payload[0]→cod`, `payload[1]→msg`); `_` ignora un slot;
  las variantes unit van desnudas (`s1`) o `s1()`:

  ```clx
  var r = resp::ok("listo");
  var out = match (v <- r) {
      ok(texto)     -> "OK ${texto}",
      err(cod, msg) -> "ERR ${cod} ${msg}",
      s1, s2        -> "unit",
      default       -> "?",
  };
  ```

- **Destructuring-let** (estático): `var resp::ok(result) = c;`. El compilador
  valida forma/variante/aridad/tipos y **exige que la variante sea demostrable**:
  el inicializador debe ser una construcción de la misma variante
  (`var c = resp::ok(x); var resp::ok(result) = c;`). Si no se puede probar →
  **error de compilación**. `_` ignora un slot. Para **clases** existe la forma
  análoga `var Clase(a, b) = obj;` sobre el payload de `__destructurate()` (ver
  `control-de-flujo.md`).
- **`==` / `is`**: `==` compara por **identidad de variante** (`def_id` + `idx`);
  `is` compara el `def_id`. Misma semántica que los enums unit-only.
- **`print` / `str`**: `variante` o `variante(payload...)`
  (p. ej. `ok(listo)`, `err(404, no)`).
- **`for each`**: solo en enums **unit-only**. Iterar un enum con payload es
  **error de compilación** (no hay payload con qué construir cada variante).

> **Gaps conocidos (F1/F2)**:
> - En modo VALOR (`match (c) in { … }`) los patrones deben ser **literales**:
>   una constante de enum (`Rojo` o `Color.Rojo`) no se acepta hoy como patrón
>   ("se esperaba un literal como patrón"). Para variantes de enum usar el modo
>   PATRÓN (`match (c) { Rojo -> … }`).
> - La construcción de una variante de un enum **importado** con nombre
>   cualificado (`mod::Enum::v(...)`, p. ej. `r::ModResp::ok("x")`) no está
>   cableada en el parser/emisor: hoy es error de sintaxis. Construir dentro del
>   propio módulo (`Enum::v(...)`) sí funciona.

### Representación (JIT/WASM)

- **Enum unit-only** (todas las variantes sin params): sigue **empacado** en i64
  `(def_id << 32) | idx` — sin heap (compatibilidad y rendimiento embebido).
- **Enum con payload** (alguna variante con params, aunque haya unit): **todos**
  sus valores son un objeto de heap
  `[def_id:i64][idx:i64][arity:i64][payload0:i64][payload1:i64]...` (cabecera de
  24 B + 8 B por slot). La distinción es **estática** (según la definición del
  enum), así que no hace falta un tag en runtime. El `def_id` es la **identidad
  de la definición** (mismo criterio que clases/structs, robusto ante imports).

El emisor abstrae `get-variant-index(valor)` y `get-payload[i](valor)` para que
`match`/`is`/`==`/`print` funcionen uniformemente con ambas representaciones.
