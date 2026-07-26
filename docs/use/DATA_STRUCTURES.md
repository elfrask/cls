# Estructuras de Datos

## Arrays

```ccls
var numbers: Int[] = [1, 2, 3, 4, 5];
var empty: String[] = [];

# Acceso
print(numbers[0]);      # 1
print(numbers[2]);      # 3

# Longitud
print(len(numbers));     # 5
```

## Records (Objetos)

```ccls
var user = {
    name: "John",
    age: 30,
    active: true
};

# Acceso
print(user.name);       # John
print(user["name"]);     # John
```

## Estructuras

Definen la forma de un record con valores por defecto:

```ccls
structure Person {
    name: String = "",
    age: Int = 0,
    country: String = "Unknown"
}

var p: Person = Person({name: "Ana", age: 25});
print(p.name);   # Ana
print(p.country); # Unknown (default)
```

## CMX (JSX nativo)

CLS soporta sintaxis tipo JSX para generar estructuras de datos:

```ccls
var app = (
    <App>
        <Header title="Mi App" />
        <Body>
            Hello World
        </Body>
    </App>
)

# Regla: <tag> minúscula → string tag
#        <Tag> mayúscula → referencia a variable
```

**Nota:** CMX genera estructuras de datos, no UI. Módulos externos
consumen estas estructuras para renderizar interfaces.
