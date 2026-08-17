# Clases y programación orientada a objetos

CLS soporta clases con herencia simple, visibilidad, campos estáticos y
`readonly`, y métodos mágicos para personalizar operadores e intrinsics.

## Declaración

Una clase se declara con `class Nombre { ... };`. El **constructor es la
función `main`** dentro de la clase: se invoca al instanciar
(`Perro("Rex")`).

```clx
class Animal {
    var nombre: String;
    function main(nombre: String) {
        me.nombre = nombre;
    }
    function hablar() -> String {
        return me.nombre + " hace ruido";
    }
};
```

- Los campos se declaran como `var nombre: Tipo;` (pueden tener inicializador:
  `var tasa: float = 0.05;`).
- Los métodos no llevan modificador de acceso por defecto (`public`).
- Las clases se cierran con `};`.

## Herencia

```clx
class Perro: Animal {
    function main(nombre: String) { me.nombre = nombre; }
    function hablar() -> String {
        return super.hablar() + " y ladra";
    }
};
```

- Sintaxis principal: `class Hijo: Padre { ... }`. También se aceptan los
  alias `extends Padre` y `(Padre)`.
- La cadena es simple: `ClassDef.ancestors: Vec<String>` = `[padre, abuelo, ...]`.
  No existe herencia múltiple (`class C: A: B` no es válido).
- Se heredan métodos, campos, constructor y visibilidad del padre.

## `me` y `super`

- `me` es el equivalente a `this`: solo existe dentro de métodos.
- `super.methodo(args)` llama al método de la clase padre con `me` = el objeto
  actual (sin recursión por override).
- `super.main(args)` ejecuta el constructor del padre.
- `super.campo` lee el campo del padre.

## Operador `is`

```clx
var d = Perro("Rex");
d is Perro;    # true (instancia directa)
d is Animal;   # true (por herencia, `Animal` está en ancestors de Perro)
d is String;   # false
```

- `true` si la clase del objeto coincide o la clase de la derecha está en
  `ancestors` de la clase del objeto.
- A la derecha debe ir una clase, struct o enum de usuario (no tipos builtin).
- Con structs/enums compara el `def_name` (ver `estructuras.md` y `enums.md`).

## Visibilidad

| Modificador | Regla |
|---|---|
| `private` | Solo accesible vía `me.` / `super.` (desde dentro de la clase) |
| `protected` | Accesible desde la clase y sus subclases; nunca desde fuera |
| `public` | Accesible desde fuera (default) |
| `static` | Vive en la definición de clase: `Clase.miembro`; los métodos estáticos no tienen `me` |
| `readonly` | Lectura externa permitida; escritura solo interna con `me.campo = x` |

```clx
class Cuenta {
    private var saldo: float;
    public var titular: String;
    protected var numero: String;
    readonly var creadoEn: int;
    static var tasa: float = 0.05;

    function main(titular: String, numero: String) {
        me.saldo = 0.0;
        me.titular = titular;
        me.numero = numero;
        me.creadoEn = 2024;
    }
    public function depositar(monto: float) {
        me.saldo = me.saldo + monto;
    }
    public function verSaldo() -> float {
        return me.saldo;
    }
    private function auditar() -> bool { return me.saldo >= 0.0; }
    protected function verNumero() -> String { return me.numero; }
    public function chequear() -> bool { return me.auditar(); }
};

function main(args: String[]) -> int {
    var c = Cuenta("Ana", "ES1234");
    c.depositar(100.5);
    print("saldo:", c.verSaldo());
    print("titular:", c.titular);        # public -> ok
    print("creadoEn:", c.creadoEn);      # readonly -> lectura ok
    print("tasa:", Cuenta.tasa);         # static vía la clase
    return 0;
};
```

El acceso se valida en `evaluate_member_access`, `evaluate_call` y
`write_target` (runtime) y en el typeck (`clx check`).

## Magic methods

Los objetos pueden personalizar intrinsics y operadores definiendo métodos con
nombres `__xxx` dentro de la clase. **Catálogo completo soportado por el JIT
(24/24)**:

| Magic method | Se invoca con |
|---|---|
| `__repr` / `__toString` | `print(obj)` / conversión a String |
| `__len` | `len(obj)` |
| `__int` / `__float` / `__bool` | `int(obj)` / `float(obj)` / `bool(obj)` |
| `__type` | `type(obj)` |
| `__toJson` | `json.stringify(obj)` |
| `__call` | `obj(args)` - objetos callables |
| `__iter` / `__next` | `for each x in (obj)` |
| `__get` / `__set` | `obj[i]` / `obj[i] = v` |
| `__contains` | `x in obj` |
| `__equals` / `__compare` | `==` `!=` / `<` `>` `<=` `>=` sobre objetos |
| `__add` `__sub` `__mul` `__div` `__mod` `__pow` | operadores aritméticos |
| `__neg` / `__not` | unario `-` / `!` |

> Catálogo detallado con contratos por intérprete: `agent-context/magics.md`.

### Ejemplo integral (JIT)

```clx
class Vector {
    var x: int;
    var y: int;
    function main(x: int, y: int) { me.x = x; me.y = y; }
    function __add(o: Vector) -> Vector { return Vector(me.x + o.x, me.y + o.y); }
    function __equals(o: Vector) -> bool { return me.x == o.x && me.y == o.y; }
    function __compare(o: Vector) -> int {
        if (me.x < o.x) { return -1; }
        if (me.x > o.x) { return 1; }
        return 0;
    }
    function __repr() -> String {
        return "Vector(" + toString(me.x) + "," + toString(me.y) + ")";
    }
};

class Rango {
    var max: int;
    var i: int = 0;
    function main(max: int) { me.max = max; }
    function __iter() -> Rango { return me; }
    function __next() -> int {
        if (me.i >= me.max) { return null; }   # null termina la iteración
        var v = me.i;
        me.i = me.i + 1;
        return v;
    }
};

function main(args: String[]) -> int {
    var a = Vector(1, 2);
    var b = Vector(3, 4);
    print(a + b);        # Vector(4,6)  - __add
    print(a == Vector(1, 2));  # true      - __equals
    print(a < b);        # true           - __compare
    for each x in (Rango(3)) { print(x); }  # 0 1 2 - __iter/__next
    return 0;
}
```

Verificado en `examples/audit/test-features/tests/jit-magic-all.clsx` (24
magics: aritmética, unarios, conversiones, contenedores, callable, iteración).

### Contratos de comportamiento

- **Aritmética binaria** (`__add` … `__pow`): `left.__op(right)`; si el lado
  izquierdo no lo define, se prueba `right.__op(left)` (simetría, paridad
  walker).
- **Igualdad y orden**: `__equals` -> el resultado se usa como truthy (`!=`
  lo niega); `__compare` -> devuelve un entero (`<0`, `0`, `>0`) comparado
  contra 0 según el operador.
- **Iteración** (`__iter`/`__next`): `__iter()` puede devolver un **Array**
  (se itera directamente) o un **objeto iterador** con `__next()` que devuelve
  los valores y `null` al terminar.
- **Indexado** (`__get`/`__set`): `obj[i]` llama `__get(index)`; `obj[i] = v`
  llama `__set(index, value)` y re-escribe el objeto en la variable (write-back).

### Notas del JIT

- **Anotación obligatoria**: los magic methods deben anotar su tipo de retorno
  (y el de los parámetros de clase). El JIT despacha estáticamente por la firma
  declarada del método - un parámetro sin anotar tipa `Any` y no se puede
  despachar (el walker sí lo tolera).
- **Sentinel de iteración**: `return null` dentro de `__next` termina el `for
  each` con un sentinel interno distinto de `0` - un iterador puede devolver
  `0` como valor legítimo (p. ej. `Rango(3)` -> `0 1 2`).
- **Mutación de arrays en campos**: `me.items.push(x)` re-escribe el array
  (que puede reallocarse) en el campo automáticamente.
- **Campos**: el JIT zero-inicializa los campos al instanciar; el inicializador
  `var items: int[] = []` no se ejecuta - inicializar en el constructor
  (`function main() { me.items = []; }`).

Referencia de ejemplos: `examples/audit/features/09-clases.clsx`,
`examples/audit/features/16-magic-methods.clsx` y
`examples/audit/test-features/tests/jit-magic-all.clsx`.
