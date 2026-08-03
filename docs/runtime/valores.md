# Valores

Todos los valores de CLS en runtime son una variante del enum `Value`
(`cls-runtime/src/value.rs`).

## El enum `Value`

```
Value::Int(i64)
Value::Float(f64)
Value::String(String)
Value::Bool(bool)
Value::Char(char)
Value::Null
Value::Void
Value::Array(Vec<Value>)
Value::Tuple(Vec<Value>)           // inmutable
Value::Record(HashMap<String, Value>)
Value::Fun(FunValue)
Value::Struct(Box<StructInstance>)
Value::Promise(Promise)
Value::Class(Box<ClassDef>)
Value::Object(Box<ClassInstance>)
Value::EnumDef(Box<EnumDef>)
Value::Enum(Box<EnumValue>)
Value::Cmx(Box<CmxValue>)
Value::Unknown
```

## Primitivos vs colecciones vs objetos

- **Primitivos planos**: `Int`, `Float`, `String`, `Bool`, `Char`, `Null`,
  `Void`. Son datos por valor, sin boxing.
- **Colecciones por valor**: `Array`, `Tuple`, `Record`. Son contenedores pero
  se resuelven como primitivos (sus métodos viven en el tipo, no en el dato).
- **Objetos/entidades**: `Object` (instancia de clase), `Struct`, `Class`,
  `Promise`, `Fun`, `EnumDef`, `Cmx`. Tienen campos o comportamiento propio.

Los `Array` son *mutable*: los mutadores (`push`, `pop`, ...) modifican el array
en el lugar. Los `Tuple` son *inmutables*: asignar `t[0] = x` es error.

## Métodos de `Value`

- `type_name() -> &str` — el nombre del tipo (`"Int"`, `"String"`, ...).
- `is_truthy() -> bool` — para condiciones: `0`, `0.0`, `""`, arrays/records
  vacíos y `null` son falsy.
- `to_string() -> String` — representación textual (los objetos usan su clase;
  los enums usan el nombre de la variante).
- `PartialEq` — igualdad por valor (los enums comparan por identidad:
  definición + índice).

## Funciones

`FunValue` es el valor de función:

- `FunKind::Native { func }` — closure de Rust.
- `FunKind::User { params, body, closure }` — función definida en CLS, con
  entorno léxico capturado opcional (closures).

## Structs

`StructInstance { def_name, fields: Vec<Value> }` — representación plana por
posición (como un struct de C). El constructor se registra con el nombre del
struct.

## Promesas

`Promise` es el puente de async: envuelve un `Pollable` (corrutina) o un
resultado ya resuelto. Se resuelve con `poll`.

## Enums

- `EnumDef { name, variants }` — la definición.
- `EnumValue { def_name, variant, index }` — una variante. El `index` es lo que
  se compila a 1-2 bytes.

## Interoperabilidad

Los valores se clonan al asignarse (semántica de valor). Los objetos y arrays
comparten la misma instancia en el intérprete (los mutadores de array escriben
de vuelta en la variable mediante *write-back*).
