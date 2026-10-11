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
`export class`, `export enum`, `export structure`, `export interface`,
`export alias`, `export extension`.
El JIT aplana el módulo importado en el módulo único y solo los símbolos
marcados `export` quedan visibles bajo el prefijo del alias.

### `export { ... };` (lista de exportación)

Forma adicional (estilo TypeScript) para exportar símbolos ya declarados
(locales o importados), con renombrado opcional. Es **aditiva**: el
`export <decl>` sigue funcionando igual.

```clx
class Caja { ... }              # sin prefijo `export`
const VERSION = "1.0";
function helper() -> int { return 1; };

export { Caja, VERSION, helper as help };
```

Los elementos pueden ser:

| Forma | Origen | `as` |
|-------|--------|:----:|
| `export { Foo }` | declaración local, binding de `from-import`, o export directo | opcional |
| `export { Foo as Bar }` | idem, renombrado | opcional |
| `export { ns::Foo as Bar }` | miembro de `import "..." as ns` | **obligatorio** |
| `export { ns::* }` | todo el contenido de `ns`, aplanado | — |
| `export { a, b as c } from "mod"` | reexport desde otro módulo | opcional |

Reglas:
- La lista no puede estar vacía.
- No se puede exportar el mismo nombre dos veces (incluye colisiones de
  `ns::*` con locales o exports directos).
- `ns::member` requiere `as` (la exportación es plana).

Módulos índice (barrel): un `.clsx` que solo reexporta otros módulos.

```clx
# framework/index.clsx
import "framework/router" as router;
import "framework/request" as request;

export { router::* };                       # todo router, aplanado
export { request::Request as Request };     # un miembro, renombrado
```

```clx
# framework/reexport.clsx (forma `from`)
export { Router, Request } from "framework/router";
```

El consumidor importa con la sintaxis ya existente:

```clx
from "framework/index" import Router, Request;
```

> Nota (limitación conocida): el **renombrado `as` de un tipo nombrado**
> (`export { Clase as Otra }`, `enum As E`) hereda la limitación del aplanado:
> el type map conserva el nombre original de la declaración. El `as` funciona
> para funciones y variables/constantes. Las clases/enums/structs/interfaces se
> reexportan con su **mismo nombre** (`export { Caja }`, `ns::Caja as Caja`).

### FFI en el sistema de módulos

Las declaraciones de `extension` (funciones nativas) son tratadas como el
**equivalente nativo del lenguaje** (`function`), así que participan del sistema
de módulos con las mismas reglas:

```clx
# ffi.clsx (cross-OS)
when (os: windows) {
    extension "msvcrt.dll" as C { export function strlen(s: CString) -> CInt; };
};
when (os: linux) {
    extension "libc.so.6" as C { export function strlen(s: CString) -> CInt; };
};
```

```clx
# consumidor
from "ffi" import strlen;          # import normal
import "ffi" as f;                 # namespacing: f::strlen(...)
# o reexport (barrel):  export { f::strlen as strlen };
```

Funciona: `export function ffiFn`, `export { ffiFn }` (promoción), reexport por
namespace/`from`, y el patrón `when`+`extension` cross-OS (se transporta al
módulo fusionado). **Limitación conocida**: el renombrado `as` de una función
nativa (`export { strlen as len }`) aún no está soportado (requiere un wrapper
con la ABI de retorno).

## Resolución de módulos

El `ModuleResolver` del runtime consulta en orden:
**caché -> internals -> hook externo -> error**.

1. **Caché** - módulos ya cargados en esta ejecución.
2. **Internals** del core: `math`, `json`. Del nodo desktop: `fs`, `http`,
   `strings` (los módulos `os`, `path`, `process`, `time`, `random` se
   eliminaron en dev-2).
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

Para módulos que requieren bindings nativos del SO (sockets, threads,
ficheros de bajo nivel, etc.), ver [extension-when.md](extension-when.md):
el patrón `when` + `extension` permite declarar esos bindings en un
`.clsx` sin necesidad de recompilar cls-core o cls-jit.

## Módulos en el JIT

El JIT **aplana** los imports: compila el entry y todos sus módulos en **un
solo** módulo WASM (`flatten_imports`), fusionando los spans con offsets de
línea únicos. Los exports de cada import quedan namespaced bajo prefijos
`alias::`. Editar cualquier `.clsx` del grafo invalida el caché.

## `module` y `namespace`

Existen las declaraciones de agrupación `module Nombre { ... };` y
`namespace Nombre { ... };`. En runtime ambas ejecutan el body en un scope
aislado y registran los símbolos definidos como un `Record` con el nombre dado
en el scope actual (son equivalentes en comportamiento), accesibles luego vía
`Nombre.miembro` / `Nombre::miembro`.

Ejemplos completos: `examples/jit-examples/modules/src/`.