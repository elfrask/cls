# Módulos

CLS tiene dos sistemas de módulos ortogonales: módulos fuente (`.clsx`) y
librerías compiladas (`.clslib`).

## Sistema A: módulos fuente

### Exportar

Los símbolos de un módulo se exponen con `export`:

```
export function doble(x: int) -> int { return x * 2; };
export var VERSION = "1.0.0";
export enum Color { Rojo, Verde };
export class Contador {
    var valor: int = 0;
    function obtener() -> int { return me.valor; }
};
export structure Par { a: int, b: int };
```

Son exportables: `function`, `var`, `const`, `class`, `enum`, `structure`.
Los símbolos sin `export` quedan privados al módulo.

### Importar

```
import "lib" as lib;

print(lib.doble(4));         // 8
print(lib.VERSION);          // "1.0.0"
print(lib.Color.Rojo);       // "Rojo"
var c = lib.Contador(0);
var p = lib.Par(1, 2);
```

- `import "path" as alias` importa todo el módulo bajo un alias.
- El alias es opcional: `import "lib"` usa el nombre del path como alias.
- El nodo resuelve `path` como `path.clsx` (relativo al directorio de trabajo).

### from import

```
from "colores" import Color;

var c: Color = Color.Rojo;
```

Importa símbolos específicos. En el verificador, los tipos del módulo se
registran de todos modos como prelude.

### Cómo se carga un módulo

Cuando el intérprete encuentra `import "lib"`, el runtime:

1. Pregunta al `ModuleResolver` (configurado por el nodo).
2. El resolver busca en la caché, luego en los módulos internos (`math`, `json`,
   `async`, y los del nodo `fs`, `http`, `Lib`), luego delega en el hook externo
   del nodo (que lee el archivo).
3. El runtime compila y ejecuta el módulo en un scope aislado
   (`Interpreter::load_module_source`).
4. Devuelve SOLO los símbolos marcados `export`, como un record.

El core y el runtime son agnósticos: no saben de dónde salen los módulos. El
nodo provee el resolver (cómo conseguirlos) y los internos del nodo.

### Verificación de tipos multi-módulo

`clx check` resuelve los imports de un archivo (y los imports de los módulos
importados), los carga como AST y los pasa al verificador como *prelude*. Así,
un tipo importado es usable en anotaciones:

```
import "colores" as colores;

var c: Color = Color.Rojo;   // 'Color' viene del prelude
```

La búsqueda de módulos es relativa al directorio del archivo que se verifica.

## Sistema B: librerías compiladas (planeado)

`Lib.load("./lib.clslib")` carga una librería compilada:

- El `.clslib` es un zip que contiene binarios `.clbin` (WASM).
- Resuelto por el `ClsLibResolver` (separado, configurable por nodo).
- Equivale a un `.dll`/`.so` para CLS.
- Va junto al `.clsapp`, no dentro.
- Busca en: directorio de trabajo, `$CLS_LIB_PATH` y las rutas del nodo.

## Módulos en línea

`module` y `namespace` declaran un record aislado:

```
module utilidades {
    export function saludar() -> String { return "hola"; }
};

print(utilidades.saludar());
```

El cuerpo se ejecuta en un entorno aislado y sus símbolos se recogen como un
record.

## Empaquetado (planeado)

Al compilar, el resolver se usa para descubrir todos los módulos que se
empaquetarán en el `.clsapp`/`.clslib`. Los módulos fuente se serializan como
AST dentro del paquete; los módulos de librería como `.clbin`.
