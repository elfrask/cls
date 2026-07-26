# Sistema de Tipos

## Type Enum

Estos son todos los tipos del lenguaje:

| Tipo | Descripción |
|------|-------------|
| `Int` | Entero (i64) |
| `Float` | Decimal (f64) |
| `String` | Cadena de texto |
| `Bool` | Booleano |
| `Char` | Carácter |
| `Any` | Cualquier tipo |
| `Unknown` | Tipo desconocido |
| `Null` | Valor nulo |
| `Void` | Sin retorno |
| `Empty` | Vacío (interno) |
| `Array(T)` | Array de tipo T |
| `Record(K, V)` | Diccionario clave K, valor V |
| `Fun(params, ret)` | Tipo función |
| `Named(name)` | Tipo definido por usuario |

## Acrónimos

| Escritura | Tipo real |
|-----------|-----------|
| `int` | `Int` |
| `str` | `String` |
| `float` | `Float` |
| `bool` | `Bool` |
| `char` | `Char` |
| `i32`, `i64`, `i16`, `i8` | `Int` |
| `f32`, `f64` | `Float` |
| `Integer` | `Int` |
| `String` | `String` |
| `Boolean` | `Bool` |
| `Character` | `Char` |
| `Any` / `any` | `Any` |
| `cmx` / `Cmx` | `Cmx` |

## Reglas de asignabilidad

- `Any` es compatible con todo
- Enteros se promueven implícitamente a `Float`
- Acrónimos (`i32`, `i64`) son compatibles con `Int`
- Arrays, Records y Funciones usan chequeo estructural
- Tipos nombrados requieren mismo nombre y parámetros compatibles

## Configuración de types

En `module.clsconfig`:

```json
{
  "compiler": {
    "types": {
      "check": true,
      "strict": false,
      "noImplicitAny": false,
      "nullSafety": true
    }
  }
}
```

- `check: false` → modo dinámico (todo `Any`)
- `strict: true` → tipos deben coincidir exactamente
- `noImplicitAny` → error si no se puede inferir tipo
- `nullSafety` → advertir sobre posibles nulos
