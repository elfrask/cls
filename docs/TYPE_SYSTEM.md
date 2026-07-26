# CLS 2.0 — Sistema de tipos

El sistema de tipos es **configurable por módulo** mediante `module.clsconfig`.

---

## Type enum

```rust
enum Type {
    // Primitivos
    Int, Float, String, Bool, Char,
    Any, Unknown, Null, Void, Empty,

    // Compuestos
    Array(Box<Type>),                    // type[]
    Record(Box<Type>, Box<Type>),       // String{Integer}
    Fun(Vec<Type>, Box<Type>),          // (Int, String) -> Bool

    // Acrónimos
    I32, I64, I16, I8,
    F32, F64, Cmx,

    // Tipos nombrados
    Named(String, Vec<Type>),           // Persona, Array<String>

    // Inferencia
    Infer(usize),
}
```

## Flags de configuración

| Flag | Default | Descripción |
|------|---------|-------------|
| `types.check` | `true` | Habilita type checker (false = modo dinámico, todo Any) |
| `types.strict` | `false` | Error si tipo no coincide exactamente |
| `types.noImplicitAny` | `false` | Prohibir tipos no inferidos |
| `types.nullSafety` | `true` | Prevenir null pointer exceptions |

## Reglas de asignabilidad

- `Any` es compatible con todo
- Enteros se promueven a Float implícitamente
- Acrónimos (`i32`, `i64`) son compatibles con `Int`
- Arrays, Records y Funciones usan chequeo estructural
- Tipos nombrados requieren mismo nombre + parámetros compatibles
- Con `strict: true`, se requiere coincidencia exacta

## Tipos acrónimos

| Acrónimo | Tipo real |
|----------|-----------|
| `int`, `Integer` | Int |
| `str`, `String` | String |
| `float`, `Float` | Float |
| `bool`, `Boolean` | Bool |
| `char`, `Character` | Char |
| `any`, `Any` | Any |
| `fun(...)`, `Function` | Fun |
| `cmx` | Cmx |
| `i32`, `i64`, `i16`, `i8` | Int (con tamaño) |
| `f32`, `f64` | Float (con tamaño) |
