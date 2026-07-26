# Clases, Módulos e Imports

## Clases

```ccls
class Person {
    private var id: Int = 0;
    export var name: String;

    function main(name: String) {
        me.name = name;
    }

    function greet() -> String {
        return "Hello, " + me.name;
    }

    static function species() -> String {
        return "Human";
    }
}

var p: Person = Person("John");
print(p.greet());
print(Person::species());
```

**Modificadores**:
- `private`: solo accesible dentro de la clase
- `export`: accesible desde fuera
- `static`: método de clase, no de instancia
- `me`: referencia a la instancia (this/self)

## Módulos

```ccls
module Utils {
    function double(x: Int) -> Int {
        return x * 2;
    }
}

# Acceso con ::
var result = Utils::double(21);
```

## Namespaces

```ccls
namespace Config {
    var version = "1.0";
    var debug = false;
}

print(Config::version);
```

## Imports

```ccls
# Importar módulo completo
import "math" as m;
m.abs(-5);

# Importar miembros específicos
from "json" import parse, stringify;
var obj = parse('{"key": "value"}');

# Importar solo un miembro con alias
from "math" import sqrt as raiz;
raiz(16);

# Incluir todos los miembros
include "math";
abs(-10);   # disponible sin prefijo
```

## Módulos disponibles

| Módulo | Acceso | Funciones |
|--------|--------|-----------|
| `math` | `math.abs()`, `math.sqrt()`, ... | Matemáticas (siempre disponible) |
| `json` | `json.parse()`, `json.stringify()` | JSON (siempre disponible) |
| `fs` | `fs.readFile()`, `fs.writeFile()`, ... | Filesystem (solo nodo desktop) |
| `http` | `http.get()`, `http.post()` | HTTP client (solo nodo desktop) |
