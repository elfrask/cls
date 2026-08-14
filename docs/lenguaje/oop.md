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
    print("titular:", c.titular);        # public → ok
    print("creadoEn:", c.creadoEn);      # readonly → lectura ok
    print("tasa:", Cuenta.tasa);         # static vía la clase
    return 0;
};
```

El acceso se valida en `evaluate_member_access`, `evaluate_call` y
`write_target` (runtime) y en el typeck (`clx check`).

## Magic methods

Los objetos pueden personalizar intrinsics y operadores definiendo métodos con
nombres `__xxx` dentro de la clase. Catálogo completo (tree-walker):

| Magic method | Se invoca con |
|---|---|
| `__repr` / `__toString` | `print(obj)` / conversión a String |
| `__len` | `len(obj)` |
| `__int` / `__float` / `__bool` | `int(obj)` / `float(obj)` / `bool(obj)` |
| `__type` | `type(obj)` |
| `__toJson` | `json.stringify(obj)` |
| `__call` | `obj(args)` — objetos callables |
| `__iter` / `__next` | `for each x in (obj)` |
| `__get` / `__set` | `obj[i]` / `obj[i] = v` |
| `__contains` | `x in obj` |
| `__equals` / `__compare` | `==` / `<` `>` sobre objetos |
| `__add` `__sub` `__mul` `__div` `__mod` `__pow` | operadores aritméticos |
| `__neg` / `__not` | unario `-` / `!` |

El **JIT** compila el subconjunto `__toString` / `__type` / `__toJson`
(verificado en `examples/audit/features/16-magic-methods.clsx`):

```clx
class Numero {
    var valor: int;
    function main(v: int) { me.valor = v; }
    function __toString() -> String { return "Numero(" + toString(me.valor) + ")"; }
    function __type() -> String { return "TipoNumero"; }
    function __toJson() -> String { return "{\"valor\":" + toString(me.valor) + "}"; }
};

function main(args: String[]) -> int {
    var a = Numero(5);
    print("toString fn:", toString(a));   # Numero(5)
    print("interpolacion: ${a}");          # Numero(5)
    print("type:", type(a));               # TipoNumero
    import "json" as json;
    print("json:", json.stringify(a));     # {"valor":5}
    return 0;
};
```

- Objetos **callables**: definen `__call` y se invocan como función.
- Objetos **iterables**: definen `__iter` (devuelve un Array o un iterador con
  `__next`, que retorna `null` al terminar) y se recorren con `for each`.

Referencia de ejemplos: `examples/audit/features/09-clases.clsx` y
`examples/audit/features/16-magic-methods.clsx`.
