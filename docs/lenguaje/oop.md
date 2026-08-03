# Clases y programación orientada a objetos

## Declaración de clase

```
class Persona {
    var nombre: String;
    var edad: int = 0;

    function main(nombre: String, edad: int) {
        me.nombre = nombre;
        me.edad = edad;
    }

    function saludar() -> String {
        return "Hola, " + me.nombre;
    }

    static function crear(nombre: String) -> Persona {
        return Persona(nombre, 0);
    }
};
```

- El método `main` es el **constructor**. Se invoca al instanciar: `Persona("Ana", 30)`.
- `me` es el `this` de CLS: referencia al objeto actual.
- Los campos se declaran con `var`; el constructor los asigna con `me.campo`.
- `static` crea métodos/fields asociados a la clase, no a la instancia.

## Instanciación

```
var p = Persona("Ana", 30);
print(p.saludar());
print(Persona.crear("Bea"));   // método estático
```

## Herencia

```
class Animal {
    var nombre: String;
    function main(nombre: String) { me.nombre = nombre; }
    function hablar() -> String { return me.nombre + " hace ruido"; }
};

class Perro: Animal {     // ':' es la sintaxis principal
    function hablar() -> String {
        return super.hablar() + " y ladra";
    }
};
```

- `class Hijo: Padre` (principal). También `extends Padre` o `(Padre)`.
- Se heredan métodos, campos y el constructor.
- `super` accede al padre:
  - `super.metodo(...)` — el método del padre (sin el override).
  - `super.campo` — un campo del padre.
  - `super.main(...)` — ejecuta el constructor del padre.

## Operador `is`

```
var d = Perro("Rex");
print(d is Perro);     // true (instancia directa)
print(d is Animal);    // true (por herencia)
print(d is String);    // false
```

`is` valida si un objeto es instancia de una clase o de una clase ancestro.

## Visibilidad

```
class Cuenta {
    private var saldo: float;
    public var titular: String;
    protected var numero: String;
    readonly var creadoEn: int;
    static var tasa: float = 0.05;

    private function auditar() -> bool { return me.saldo >= 0; }
    protected function verNumero() -> String { return me.numero; }
    public function depositar(monto: float) { me.saldo = me.saldo + monto; }
    static function obtenerTasa() -> float { return Cuenta.tasa; }
};
```

| Modificador | Acceso |
|-------------|--------|
| `private` | Solo desde dentro de la clase (vía `me.`/`super.`). |
| `protected` | Desde la clase y sus subclases. Nunca desde fuera. |
| `public` | Desde cualquier parte. |
| `static` | Vive en la clase; se accede con `Clase.miembro`. |
| `readonly` | Lectura externa permitida; escritura solo interna (`me.campo = ...`). |

El verificador e intérprete hacen cumplir estas reglas: acceder a un miembro
`private`/`protected` desde fuera produce error en runtime.

## Magic methods

Los objetos pueden definir métodos con doble guion bajo que el intérprete invoca
en operaciones concretas. El método mágico se llama `__nombre` (sin sufijo).

```
class Numero {
    var valor: int;
    function main(v: int) { me.valor = v; }
    function __toString() -> String { return "Numero(" + toString(me.valor) + ")"; }
    function __equals(other) -> bool { return me.valor == other.valor; }
    function __add(other) -> Numero { return Numero(me.valor + other.valor); }
    function __len() -> int { return 1; }
    function __call(x: int) -> int { return me.valor + x; }
};
```

Catálogo:

| Magic method | Se activa con |
|--------------|---------------|
| `__toString()` | `print(x)`, `toString(x)`, interpolación. |
| `__equals(other)` | `==` y `!=`. |
| `__compare(other)` | `<`, `<=`, `>`, `>=` (devuelve -1/0/1). |
| `__add` / `__sub` / `__mul` / `__div` / `__mod` / `__pow` | Operadores aritméticos. |
| `__neg()` / `__not()` | Unarios `-` y `!`. |
| `__int()` / `__float()` / `__bool()` | `int(x)`, `float(x)`, `bool(x)`. |
| `__len()` | `len(x)`. |
| `__get(index)` | `x[i]` (lectura). |
| `__set(index, value)` | `x[i] = v` (escritura). |
| `__contains(value)` | `value in x`. |
| `__iter()` / `__next()` | `for each item in (x)`. |
| `__call(...)` | `x(...)` (objeto invocable). |
| `__type()` | `type(x)` (devuelve un nombre de tipo custom). |
| `__toJson()` | `json.stringify(x)`. |
| `__clone()` | Método normal que el usuario llama explícitamente. |

Si el magic method no existe, se usa el comportamiento por defecto (sin error).

## Genéricos en clases

```
class Caja<T> {
    var contenido: T;
    function main(contenido: T) { me.contenido = contenido; }
    function obtener() -> T { return me.contenido; }
};
```

- Los campos y métodos usan `T` como parámetro de tipo.
- En runtime, `Caja(42)` funciona sin anotación.
- La verificación completa de `Caja<String>` (que los miembros usen `T=String`)
  en el verificador está planeada como mejora futura.
