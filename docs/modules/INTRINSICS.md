# Funciones Intrínsecas

Las funciones intrínsecas están disponibles globalmente sin importar.

## I/O

| Función | Firma | Descripción |
|---------|-------|-------------|
| `print(values...)` | `(Any...) -> Void` | Imprime en stdout |
| `input(prompt)` | `(String) -> String` | Lee de stdin |
| `args` | `String[]` | Argumentos del CLI (parámetro de main) |

## Conversión de tipos

| Función | Firma | Descripción |
|---------|-------|-------------|
| `int(val)` | `(Any) -> Int` | Convierte a entero |
| `str(val)` | `(Any) -> String` | Convierte a string |
| `float(val)` | `(Any) -> Float` | Convierte a decimal |
| `bool(val)` | `(Any) -> Bool` | Convierte a booleano |
| `toString(val)` | `(Any) -> String` | Representación string |

## Utilidades

| Función | Firma | Descripción |
|---------|-------|-------------|
| `type(val)` | `(Any) -> String` | Nombre del tipo |
| `len(val)` | `(Array | String | Record) -> Int` | Longitud |
| `now()` | `() -> Int` | Timestamp en ms |
| `exit(code)` | `(Int) -> Void` | Termina el programa |
| `sleep(ms)` | `(Int) -> Void` | Pausa en milisegundos |

## Uso

```ccls
function main(args: String[]) -> int {
    var name = input("Nombre: ");
    print("Hola,", name);
    print("Args:", args);

    var num = int("42");        # 42
    var text = str(3.14);       # "3.14"
    var flag = bool(1);         # true
    var currentTime = now();    # 1785095589093
    var size = len("hello");    # 5

    return 0;
}
```
