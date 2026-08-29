# Catálogo de Magic Methods en CLS 2.0

> Referencia de los métodos mágicos (`__nombre`) de CLS 2.0: qué hacen,
> cuándo se invocan y su estado de soporte en cada intérprete.
>
> - **JIT** (`clx run`, `clxr`, `nodos/clx/src/jit.rs` +
>   `cls-core/src/backend/wasm`) — intérprete objetivo.
> - **Walker** (`clx run --ast-walker`, `cls-runtime`) — referencia
>   sintáctica, deprecado tras CLS 2.0-dev1.
>
> Leyenda: ✅ implementado · ⚠️ parcial · ❌ no implementado.

---

## Auditoría dev-2

El catálogo original marcaba muchos magics como `❌` en el JIT, pero
estaban implementados en el emisor (a través de
`emit_class_method_args` y `try_binary_magic`) sin tests que los
ejercitaran. La auditoría los validó con
`examples/audit/test-features/tests/jit-magic-all.clsx` y
`test-magic.clsx` — **todos los magics del catálogo funcionan
end-to-end en el JIT**, salvo donde se indica explícitamente.

---

## 1. Conversión a string

| Magic | Walker | JIT | Contrato |
|-------|:------:|:---:|----------|
| `__toString` | ✅ | ✅ | `__toString() -> String` — representación canónica (usada por `toString(obj)` y por `value_to_string`). El JIT la busca en `func_indexes["Clase::__toString"]`; si no existe cae a `__repr`. |
| `__repr` | ✅ | ✅ | `__repr() -> String` — representación de impresión (prioridad sobre `__toString` en `print`). El walker usa `__repr` primero; el JIT la usa si `__toString` no existe. |

**Precedencia de impresión** (walker): `__repr` → `__toString` → formato
por defecto. **`print(obj)`** invoca `__repr` (y luego `__toString`)
por cada argumento.

```clx
class Punto {
    var x: int;
    function main(x: int) { me.x = x; }
    function __toString() -> String { return "Punto(" + str(me.x) + ")"; }
};
print(Punto(3));   // Punto(3)
```

---

## 2. Operadores aritméticos (binarios)

| Magic | Operador | Walker | JIT | Contrato |
|-------|----------|:------:|:---:|----------|
| `__add` | `+` | ✅ | ✅ | `__add(other) -> value` |
| `__sub` | `-` | ✅ | ✅ | `__sub(other) -> value` |
| `__mul` | `*` | ✅ | ✅ | `__mul(other) -> value` |
| `__div` | `/` | ✅ | ✅ | `__div(other) -> value` |
| `__mod` | `%` | ✅ | ✅ | `__mod(other) -> value` |
| `__pow` | `**` | ✅ | ✅ | `__pow(other) -> value` |

**Dispatch** (JIT, `try_binary_magic` en `emitter/classes.rs:145`):
intenta `left.__op(right)` y si no lo implementa, `right.__op(left)`
(simetría, paridad con el walker).

```clx
class Vector {
    var x: int;
    function main(x: int) { me.x = x; }
    function __add(o: Vector) -> Vector { return Vector(me.x + o.x); }
};
var a = Vector(1);
var b = Vector(2);
print(a + b);   // Vector(3)
```

---

## 3. Operadores unarios y conversión

| Magic | Operador/uso | Walker | JIT | Contrato |
|-------|--------------|:------:|:---:|----------|
| `__neg` | `-obj` | ✅ | ✅ | `__neg() -> value` — negación unaria |
| `__not` | `!obj` | ✅ | ✅ | `__not() -> bool` — si no existe, truthiness por defecto |
| `__bool` | `bool(obj)` | ✅ | ✅ | `__bool() -> bool` — conversión a booleano |
| `__int` | `int(obj)` | ✅ | ✅ | `__int() -> int` |
| `__float` | `float(obj)` | ✅ | ✅ | `__float() -> float` |

**`int`/`float`/`bool`** (JIT, `emitter/primitives.rs:582-602`):
mapean al magic correspondiente si la clase lo implementa; si no, al
intrinsic de runtime (`int()`/`float()`/`bool()`).

---

## 4. Comparación e igualdad

| Magic | Operador/uso | Walker | JIT | Contrato |
|-------|--------------|:------:|:---:|----------|
| `__equals` | `==` / `!=` | ✅ | ✅ | `__equals(other) -> bool` — igualdad semántica entre objetos |
| `__compare` | `<` `<=` `>` `>=` | ✅ | ✅ | `__compare(other) -> int` — negativo/0/positivo |
| `__contains` | `x in obj` | ✅ | ✅ | `__contains(needle) -> bool` — pertenencia |

**Dispatch de `==`** (JIT, `emitter/binary.rs:32-50`): si la clase define
`__equals`, se llama; el resultado se niega para `!=`. **`in`**
(`emitter/binary.rs:350-356`): si la clase define `__contains`, se llama;
si no, el runtime tiene su propio `__intr_str_contains` para strings.

---

## 5. Contenedores e indexado

| Magic | Uso | Walker | JIT | Contrato |
|-------|-----|:------:|:---:|----------|
| `__get` | `obj[index]` | ✅ | ✅ | `__get(index) -> value` — lectura por índice |
| `__set` | `obj[index] = v` | ✅ | ✅ | `__set(index, value)` — escritura por índice (write-back) |
| `__len` | `len(obj)` | ✅ | ✅ | `__len() -> int` — tamaño |

**`len(obj)`** (JIT, `emitter/primitives.rs:541`): si la clase define
`__len`, se llama; si no, el runtime tiene su propio `__intr_*_len`
para arrays/strings.

```clx
class Lista {
    var items: Array;
    function main() { me.items = [1, 2, 3]; }
    function __get(i: int) -> int { return me.items[i]; }
    function __set(i: int, v: int) { me.items[i] = v; }
    function __len() -> int { return len(me.items); }
};
```

---

## 6. Iteración

| Magic | Uso | Walker | JIT | Contrato |
|-------|-----|:------:|:---:|----------|
| `__iter` | `for each x in obj` | ✅ | ✅ | `__iter() -> Array \| iterador` |
| `__next` | iterador | ✅ | ✅ | `__next() -> value \| null` — null termina |

**Protocolo** (JIT, `emitter/statements.rs` handler de `ForEach`):
`__iter()` puede devolver un **Array** (se itera directamente) o un
**objeto iterador** con `__next()` que devuelve valores hasta `null`.

```clx
class Rango {
    var max: int;
    var i: int = 0;
    function main(max: int) { me.max = max; }
    function __next() -> int {
        if (me.i >= me.max) { return null; }
        var v = me.i;
        me.i++;
        return v;
    }
};
for each x in Rango(3) { print(x); }   // 0 1 2
```

---

## 7. Llamada (callable)

| Magic | Uso | Walker | JIT | Contrato |
|-------|-----|:------:|:---:|----------|
| `__call` | `obj(args...)` | ✅ | ✅ | `__call(...) -> value` — objeto invocable |

**`obj(args)`** (JIT, `emitter/calls.rs:332`): si la clase define `__call`,
se invoca; si no, error "no es callable (falta `__call`)".

```clx
class Doble {
    function __call(x: int) -> int { return x * 2; }
};
print(Doble()(5));   // 10
```

---

## 8. Serialización e introspección

| Magic | Uso | Walker | JIT | Contrato |
|-------|-----|:------:|:---:|----------|
| `__toJson` | `json.stringify(obj)` | ✅ | ✅ | `__toJson() -> value` — valor serializable (si no existe → `"null"`, paridad) |
| `__type` | `type(obj)` | ✅ | ✅ | `__type() -> String` — nombre del tipo dinámico |

**`json.stringify(obj)`**: usa `__toJson` si la clase lo define; si no,
`null`. **`type(obj)`**: usa `__type` si la clase lo define (si no, el
tipo estático).

---

## 9. Constructor y helpers internos (no magics de usuario)

| Nombre | Uso | Descripción |
|--------|-----|-------------|
| `main` (en clase) | `Clase(args)` | **Constructor** — el parser lo registra como ctor; el JIT lo emite como `Clase::__ctor` |
| `__ctor` | interno | Ctor compilado (JIT): `Clase::__ctor` con `me` como primer param |
| `__method__.` | interno | Prefijo de métodos de primitivos bound (JIT) para evitar colisión con intrinsics |
| `__alloc` / `__load_str` | interno | Funciones internas del WASM del JIT |

---

## Estado de soporte por intérprete

| Categoría | Walker | JIT |
|-----------|:------:|:---:|
| `__toString`, `__repr` | ✅ | ✅ |
| `__toJson`, `__type` | ✅ | ✅ |
| Aritmética (`__add`…`__pow`) | ✅ | ✅ |
| Unarios/conversión (`__neg`, `__not`, `__bool`, `__int`, `__float`) | ✅ | ✅ |
| Igualdad/orden (`__equals`, `__compare`, `__contains`) | ✅ | ✅ |
| Contenedores (`__get`, `__set`, `__len`) | ✅ | ✅ |
| Iteración (`__iter`, `__next`) | ✅ | ✅ |
| Callable (`__call`) | ✅ | ✅ |

**M-13 cerrado** (dev-2): todos los magic methods del catálogo
funcionan en el JIT, validados con `jit-magic-all.clsx` y
`test-magic.clsx` (con tipos anotados en los parámetros).

### Notas para el usuario

- El magic recibe el `other`/`index`/`value` con tipo **anotado** en
  la firma. Si no se anota, el typeck infiere `Any` y la coerción
  posterior puede dar resultados inesperados en primitivos. Anota
  siempre: `function __add(other: MiClase) -> MiClase { ... }`.
- Para `__set` con semántica de write-back, el JIT lee el campo vía
  `__get`, llama al método, y reasigna el resultado en el slot del
  objeto en memoria. Si tu clase tiene campos por valor, considera
  documentar explícitamente la semántica de copia vs referencia.
- Los magics heredan por la cadena de `ancestors`: `Base::__add` se
  llama si `Hijo` no la redefine (resuelto en
  `class_magic_method` en `emitter/classes.rs`).

### Cómo agregar un nuevo magic

1. Documentar en este catálogo con: nombre, contrato (firma +
   retorno), dispatcher (cuándo se invoca), ejemplo de uso.
2. El typeck necesita el helper para detectar el magic en el span del
   binary/unary/llamada y validar el tipo del operando:
   `named_magic_ret` en `cls-core/src/middleware/typeck/magics.rs`.
3. El emisor necesita el dispatch al método de clase por nombre. Para
   binarios usar `try_binary_magic`; para unarios/llamadas/indexado
   usar `emit_class_method_args` o `emit_class_method_call_on`
   directamente.
4. Test E2E: agregar caso en
   `examples/audit/test-features/tests/jit-magic-all.clsx`.
