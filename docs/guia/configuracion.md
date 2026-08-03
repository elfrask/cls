# Configuración: cls.json

`cls.json` es el manifiesto único de un proyecto CLS. Unifica los metadatos del
proyecto, la configuración del compilador y la del intérprete en un solo
archivo. Se busca desde el directorio de trabajo hacia arriba; si no existe, se
usan los valores por defecto.

## Ejemplo completo

```json
{
  "name": "mi-proyecto",
  "version": "0.1.0",
  "description": "Un proyecto CLS",
  "authors": ["Nombre <correo>"],
  "license": "MIT",
  "registry": "https://registry.cls-lang.org",
  "entry": "src/main.clsx",

  "project": {
    "sourceDir": "src",
    "outDir": "dist",
    "target": "executable"
  },

  "compiler": {
    "targetArchitecture": "wasm",
    "optimizationLevel": "O2",
    "sourceMaps": true,
    "types": {
      "check": true,
      "strict": false,
      "noImplicitAny": false,
      "nullSafety": true
    },
    "features": {
      "async": true,
      "macros": false,
      "experimental": false
    },
    "warnings": {
      "unusedVariables": "warn",
      "deadCode": "warn",
      "treatWarningsAsErrors": false
    }
  },

  "interpreter": {
    "optimization": false,
    "mode": "pure-ast",
    "runtime": {
      "memoryLimit": "512MB",
      "stackSize": "8MB",
      "gc": {
        "enabled": false,
        "strategy": "compiled",
        "threshold": "64MB"
      }
    },
    "sandbox": {
      "allowFs": false,
      "allowNet": false,
      "maxExecutionTime": 5000
    }
  },

  "dependencies": {},
  "devDependencies": {}
}
```

## Campos del proyecto

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `name` | — | Nombre del proyecto (requerido). |
| `version` | `0.1.0` | Versión semver. |
| `description` | `""` | Descripción. |
| `authors` | `[]` | Lista de autores. |
| `license` | `MIT` | Licencia. |
| `registry` | `https://registry.cls-lang.org` | URL del registry de dependencias. |
| `entry` | `src/main.clsx` | Punto de entrada principal. |
| `dependencies` / `devDependencies` | `{}` | Dependencias (nombre → versión). |

## Sección `project`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `sourceDir` | `src` | Directorio del código fuente. |
| `outDir` | `dist` | Directorio de salida del build. |
| `target` | `executable` | Tipo de artefacto: `executable`, `library`, `dynamic-lib`. |

## Sección `compiler`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `targetArchitecture` | `wasm` | Arquitectura objetivo: `wasm`, `x86_64`, `arm64`, `bytecode`. |
| `optimizationLevel` | `O2` | Nivel de optimización: `O0`..`O3`, `Os`. |
| `sourceMaps` | `true` | Generar source maps para depuración. |

### `compiler.types`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `check` | `true` | Habilitar el verificador de tipos (híbrido). Si es `false`, el lenguaje se comporta dinámico. |
| `strict` | `false` | Modo estricto: las asignaciones incompatibles son error (no advertencia). |
| `noImplicitAny` | `false` | Prohibir variables sin tipo inferible. |
| `nullSafety` | `true` | Advertir cuando se asigna `null` a un tipo no `Any`. |

### `compiler.features`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `async` | `false` | Habilitar funciones asíncronas y el módulo `async`. |
| `macros` | `false` | Habilitar macros (planeado). |
| `experimental` | `false` | Habilitar features experimentales. |

### `compiler.warnings`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `unusedVariables` | `warn` | `warn`, `error` u `off`. |
| `deadCode` | `warn` | `warn`, `error` u `off`. |
| `treatWarningsAsErrors` | `false` | Elevar todas las advertencias a errores. |

## Sección `interpreter`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `optimization` | `false` | Optimizaciones durante la interpretación. |
| `mode` | `pure-ast` | Modo de ejecución: `pure-ast` (tree-walker) o `jit` (planeado). |

### `interpreter.runtime`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `memoryLimit` | `512MB` | Límite de memoria del runtime. |
| `stackSize` | `8MB` | Tamaño de pila. |
| `gc.enabled` | `false` | Habilitar el recolector de basura. |
| `gc.strategy` | `compiled` | `active` (runtime) o `compiled` (quemado en WASM). |
| `gc.threshold` | `64MB` | Umbral de activación del GC. |

### `interpreter.sandbox`

| Campo | Por defecto | Descripción |
|-------|-------------|-------------|
| `allowFs` | `false` | Permitir acceso al sistema de archivos. |
| `allowNet` | `false` | Permitir acceso a la red. |
| `maxExecutionTime` | `5000` | Tiempo máximo de ejecución en milisegundos. |

## Notas

- Los campos usan `camelCase` en el JSON (por ejemplo `sourceDir`, `maxExecutionTime`).
- El tipo checker lee `compiler.types`; `clx check --strict` fuerza `strict: true`
  aunque el manifiesto diga lo contrario.
- Si un campo falta, se aplica su valor por defecto; el manifiesto nunca falla
  por campos ausentes.
