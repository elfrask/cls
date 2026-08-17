# Módulos, imports y exports

CLS tiene dos sistemas de módulos: los **módulos fuente** (`.clsx`, con
`import`/`from import`/`include`) y las **librerías compiladas** (`.clslib` vía
`Lib.load`, pendiente de WASM). Este documento cubre los módulos fuente.

## Imports

```clx
import "lib/mathx" as mathx;              # namespaced
from "lib/stringsx" import gritar as gritarFn, repetir;
include "lib/colores";
```

### `import "mod" as alias;`

Agrupa el módulo bajo un alias. Los símbolos exportados se acceden con
namespacing:

```clx
import "lib/mathx" as mathx;

function main(args: String[]) -> int {
    print("cuadrado:", mathx::cuadrado(5));  # NamespaceAccess (JIT)
    print("cubo:", mathx::cubo(3));
    print("PI:", mathx::PI);                  # export var
    return 0;
};
```

- `alias::funcion` (`NamespaceAccess`) es el acceso soportado por el JIT.
- `alias.funcion()` y `alias.var` (member access sobre el Record del módulo)
  funcionan en el tree-walker.
- Si no se da alias, el runtime usa el path como nombre (`import "mod"` ->
  `mod::sym`).

### `from "mod" import a as b, c;`

Trae **solo** los símbolos exportados nombrados, opcionalmente renombrados:

```clx
from "lib/stringsx" import gritar as gritarFn, repetir;

function main(args: String[]) -> int {
    print("gritar:", gritarFn("hola"));
    print("repetir:", repetir("ab", 3));
    return 0;
};
```

Solo se puede importar lo que está marcado `export`; importar algo no
exportado da error (el typeck sugiere: `el módulo exporta: ...`).

### `include "mod";`

Inyecta **todos** los exports del módulo en el scope actual, sin namespacing.
Útil para enums u otros símbolos que quieras usar desnudos:

```clx
include "lib/colores";

function main(args: String[]) -> int {
    var c = Color.Azul;      # sin prefijo
    return 0;
};
```

## Exports

Solo lo marcado `export` es visible desde otros módulos:

```clx
export function cuadrado(x: int) -> int { return x * x; };
export var PI = "3.14159";
export enum Color { Rojo, Verde, Azul, };
```

Declaraciones exportables: `export function`, `export var`/`const`,
`export class`, `export enum`, `export structure`, `export extension`.
La carga del módulo (centralizada en `Interpreter::load_module_source`)
ejecuta el archivo en un scope aislado y devuelve únicamente los símbolos
exportados.

## Resolución de módulos

El `ModuleResolver` del runtime consulta en orden:
**caché -> internals -> hook externo -> error**.

1. **Caché** - módulos ya cargados en esta ejecución.
2. **Internals** del core: `math`, `json`, `async`. Del nodo desktop:
   `fs`, `http`, `Lib`, `os`, `path`, `process`, `time`, `random`.
3. **Hook externo** - el nodo resuelve módulos de usuario; en `clx` el orden de
   candidatos es:
   1. `{dir del archivo}/{path}.clsx` (junto al archivo que importa)
   2. `{workspace}/modules/{nombre}/mod.clsx`
   3. `{cwd}/{path}.clsx`
   4. `{cwd}/modules/{nombre}/mod.clsx`
   5. `~/.cls/modules/{nombre}@{version}/mod.clsx` (globales versionadas;
      filtra por el rango semver declarado en `cls.json`)
   6. `~/.cls/modules/{nombre}/mod.clsx` (globales sin versión)
4. **Error** - `Módulo 'X' no encontrado`.

## Módulos en el JIT

El JIT **aplana** los imports: compila el entry y todos sus módulos en **un
solo** módulo WASM (`flatten_imports`), fusionando los spans con offsets de
línea únicos. Los exports de cada import quedan namespaced bajo prefijos
`alias::`. Editar cualquier `.clsx` del grafo invalida el caché.

## `module` y `namespace`

Existen las declaraciones de agrupación `module Nombre { ... };` y
`namespace Nombre { ... };`. En runtime ambas ejecutan el body en un scope
aislado y registran los símbolos definidos como un `Record` con el nombre dado
en el scope actual (son equivalentes en comportamiento; ver
`Interpreter::execute_module_decl` / `execute_namespace_decl`), accesibles
luego vía `Nombre.miembro` / `Nombre::miembro`.

Ejemplos completos: `examples/jit-examples/modules/src/`.