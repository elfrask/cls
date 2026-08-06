# Documentación de CLS

CLS es un lenguaje de programación compilado-orientado, con un intérprete
(tree-walker), un verificador de tipos (compile-time) y un plan de compilación
nativa/WASM. Esta documentación cubre el lenguaje, el runtime, la arquitectura
del proyecto y cómo contribuir.

## Estructura de la documentación

Cada tema vive en su propio documento. Empieza por `guia/inicio-rapido.md` si
no conoces el lenguaje, o por `lenguaje/arquitectura.md` para entender el
diseño.

### Guías de uso

- `guia/instalacion.md` — requisitos, compilación y ejecución.
- `guia/inicio-rapido.md` — primer programa, sintaxis esencial.
- `guia/cli.md` — todos los subcomandos de `clx` y `clxr`.
- `guia/configuracion.md` — el manifiesto `cls.json` y todas sus opciones.

### El lenguaje

- `lenguaje/arquitectura.md` — las capas, el pipeline, las extensiones de archivo.
- `lenguaje/sintaxis.md` — léxico, tipos base, variables, literales.
- `lenguaje/control-de-flujo.md` — if, while, loop, for, for-each, switch, try, break, continue.
- `lenguaje/funciones.md` — funciones, genéricos, funciones flecha, async/await.
- `lenguaje/tipos.md` — tuplas, arrays, records, uniones, alias, interfaces, extracción de tipos.
- `lenguaje/enums.md` — enums con identidad, iteración, comparación.
- `lenguaje/oop.md` — clases, herencia, visibilidad, super, magic methods.
- `lenguaje/modulos.md` — el sistema de módulos, imports, exports.
- `lenguaje/cmx.md` — el lenguaje de marcado CMX (JSX-like).
- `lenguaje/extension.md` — FFI a librerías nativas del sistema (`extension`).
- `lenguaje/multi-entorno.md` — directiva `when` (implementaciones por SO/arquitectura).
- `lenguaje/cmx.md` — el lenguaje de marcado CMX (JSX-like).

### Runtime y ejecución

- `runtime/ejecucion.md` — cómo funciona el intérprete (tree-walker).
- `runtime/valores.md` — el sistema de valores (`Value`).
- `runtime/biblioteca-estandar.md` — math, json, fs, http, async.
- `runtime/metodos-primitivos.md` — métodos de tipos primitivos (sin boxing).
- `runtime/errores.md` — sistema de errores y formatos de salida.

### Ejecución sin nodo y resolvers

- `ejecucion/sin-nodo.md` — usar el core y el runtime directamente (embedding).
- `ejecucion/resolvers.md` — cómo funciona la resolución de módulos y cómo desarrollar un resolver.

### Desarrollo y contribución

- `desarrollo/contribuir.md` — workflow, estilo, cómo reportar.
- `desarrollo/arquitectura-core.md` — cls-core en detalle (lexer, parser, middleware).
- `desarrollo/agregar-feature.md` — cómo agregar una feature al lenguaje.
- `desarrollo/agregar-modulo-interno.md` — cómo agregar un módulo interno.
- `desarrollo/testing.md` — cómo ejecutar y escribir los tests.

## Convenciones

- Los ejemplos de código usan bloques de lenguaje `clx`.
- Las rutas internas se escriben como texto (p. ej. `cls-core/src/...`) sin enlaces.
- Esta documentación se mantiene sincronizada con la implementación; si algo
  no coincide con el comportamiento real, es un error de documentación.
