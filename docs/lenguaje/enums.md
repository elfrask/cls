# Enums

Los enums son **literales con identidad**: a diferencia de un `alias` de unión
(que solo afecta al verificador), tienen valor en runtime. Cada variante es un
índice dentro del enum, por lo que al compilar se representan en 1-2 bytes
(`u8`/`u16`). Son ideales para estados en sistemas embebidos: en lugar de
comparar cadenas que consumen memoria, escribes nombres representativos que se
compilan a un índice minúsculo.

## Declaración

```
enum Color {
    Rojo,
    Verde,
    Azul,
};
```

- Las variantes son constantes con identidad única.
- Se acceden de forma namespaced: `Color.Rojo`.

## Uso

```
var c = Color.Rojo;

print(c);                 // "Rojo"
print(c == Color.Rojo);   // true
print(c == Color.Verde);  // false
print(c is Color);        // true
```

- **Comparación**: `==` compara por identidad (definición + índice).
- **`is`**: `c is Color` valida que el valor pertenece al enum.

## Iteración

Los enums son iterables:

```
for each v in (Color) {
    print(v);
}
```

Con índice:

```
for each v and i in (Color) {
    print(i, v);
}
```

## Tipado

En el verificador, un enum define un tipo con el nombre del enum:

```
var c: Color = Color.Rojo;    // ok
var d: Color = 5;             // error en estricto: Int no es Color
```

## Runtime

En el intérprete, un enum tiene dos valores:

- `Value::EnumDef` — la definición (nombre + lista de variantes).
- `Value::Enum` — una variante concreta `{ def_name, variant, index }`.

El `index` es lo que se compila a 1-2 bytes en un binario nativo.

## Notas

- Las variantes no llevan valor asociado (sin payload). La extracción de payload
  (pattern matching) está planeada como mejora futura.
- Los enums son exportables: `export enum Color { ... }` y se resuelven desde
  otros módulos con `lib.Color.Rojo`.
