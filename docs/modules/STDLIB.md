# Módulos Internos — API

## math

Módulo de funciones matemáticas. Siempre disponible.

### Funciones

| Función | Firma | Descripción |
|---------|-------|-------------|
| `abs(x)` | `(Int | Float) -> Int | Float` | Valor absoluto |
| `sqrt(x)` | `(Float) -> Float` | Raíz cuadrada |
| `pow(base, exp)` | `(Float, Float) -> Float` | Potencia |
| `min(a, b)` | `(Float, Float) -> Float` | Mínimo |
| `max(a, b)` | `(Float, Float) -> Float` | Máximo |
| `floor(x)` | `(Float) -> Int` | Redondeo hacia abajo |
| `ceil(x)` | `(Float) -> Int` | Redondeo hacia arriba |
| `round(x)` | `(Float) -> Int` | Redondeo al más cercano |
| `sin(x)`, `cos(x)`, `tan(x)` | `(Float) -> Float` | Trigonométricas |
| `log(x)` | `(Float) -> Float` | Logaritmo natural |
| `random()` | `() -> Float` | Número aleatorio 0..1 |
| `range(start, end)` | `(Int, Int) -> Int[]` | Array de inicio a fin (excl) |

### Constantes

| Nombre | Valor |
|--------|-------|
| `math.PI` | 3.141592653589793 |
| `math.E` | 2.718281828459045 |

### Uso

```ccls
import "math" as math;
math.abs(-10)       # 10
math.sqrt(16)       # 4
math.floor(3.7)     # 3
math.PI             # 3.1415...
```

---

## json

Módulo de serialización JSON. Siempre disponible.

### Funciones

| Función | Firma | Descripción |
|---------|-------|-------------|
| `parse(text)` | `(String) -> Any` | Convierte JSON string a valor CLS |
| `stringify(value)` | `(Any) -> String` | Convierte valor CLS a JSON string |

### Uso

```ccls
from "json" import parse, stringify;

var obj = parse('{"name": "CLS", "version": 2.0}');
print(obj.name);           # CLS
print(stringify(obj));     # {"name":"CLS","version":2.0}
```

---

## fs (solo nodo desktop)

Módulo de sistema de archivos. Solo disponible en el nodo `clx` (desktop).

### Funciones

| Función | Firma | Descripción |
|---------|-------|-------------|
| `readFile(path)` | `(String) -> String` | Lee archivo completo |
| `writeFile(path, content)` | `(String, String) -> Void` | Escribe archivo |
| `exists(path)` | `(String) -> Bool` | Verifica si existe |
| `rm(path)` | `(String) -> Void` | Elimina archivo/directorio |
| `mkdir(path)` | `(String) -> Void` | Crea directorio |
| `listDir(path)` | `(String) -> String[]` | Lista contenido |
| `cwd()` | `() -> String` | Directorio actual |

### Uso

```ccls
import "fs" as fs;
var content = fs.readFile("data.json");
fs.writeFile("output.txt", "Hello");
```

---

## http (solo nodo desktop)

Módulo HTTP. Solo disponible en el nodo `clx` (desktop).

### Funciones

| Función | Firma | Descripción |
|---------|-------|-------------|
| `get(url)` | `(String) -> String` | HTTP GET |
| `post(url, body)` | `(String, String) -> String` | HTTP POST |

### Uso

```ccls
import "http" as http;
var data = http.get("https://api.example.com/data");
```
