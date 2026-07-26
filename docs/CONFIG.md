# CLS 2.0 — Configuración de proyecto (`module.clsconfig`)

El manifiesto de proyecto define metadatos, configuración del compilador, intérprete,
sandbox y dependencias de un módulo CLS.

---

## Formato

```json
{
  "name": "mi-plugin",
  "version": "1.0.0",
  "description": "Un plugin/librería para mi lenguaje",
  "authors": ["Tu Nombre <tu@email.com>"],
  "license": "MIT",

  "project": {
    "entry": "src/main.ccls",
    "sourceDir": "src",
    "outDir": "dist",
    "target": "executable"
  },

  "compiler": {
    "targetArchitecture": "x86_64",
    "optimizationLevel": "O2",
    "types": {
      "check": true,
      "strict": false,
      "noImplicitAny": true,
      "nullSafety": true
    },
    "features": {
      "async": true,
      "macros": true,
      "experimental": false
    },
    "warnings": {
      "unusedVariables": "warn",
      "deadCode": "warn",
      "treatWarningsAsErrors": false
    },
    "sourceMaps": true
  },

  "interpreter": {
    "optimization": true,
    "mode": "jit",
    "runtime": {
      "memoryLimit": "512MB",
      "stackSize": "8MB",
      "gc": {
        "enabled": true,
        "strategy": "compiled"
      }
    },
    "sandbox": {
      "allowFS": false,
      "allowNet": false,
      "maxExecutionTime": 5000
    }
  },

  "dependencies": {
    "std": ">=1.0.0"
  },
  "devDependencies": {
    "test-runner": "^0.2.0"
  }
}
```

---

## Secciones

### `project`
| Campo | Valores | Descripción |
|-------|---------|-------------|
| `entry` | Path | Punto de entrada principal (default: `src/main.ccls`) |
| `sourceDir` | Path | Directorio del código fuente (default: `src`) |
| `outDir` | Path | Directorio de salida (default: `dist`) |
| `target` | `"executable"` / `"library"` / `"dynamic-lib"` | Tipo de artefacto |

### `compiler`
| Campo | Valores | Descripción |
|-------|---------|-------------|
| `targetArchitecture` | `"x86_64"` / `"arm64"` / `"wasm"` / `"bytecode"` | Arquitectura objetivo |
| `optimizationLevel` | `"O0"` / `"O1"` / `"O2"` / `"O3"` / `"Os"` | Nivel de optimización |
| `sourceMaps` | bool | Generar mapas de debugging |

### `interpreter`
| Campo | Valores | Descripción |
|-------|---------|-------------|
| `optimization` | bool | Optimizar en interpretación |
| `mode` | `"pure-ast"` / `"jit"` | Modo de ejecución |
| `runtime.memoryLimit` | `"512MB"` | Límite de memoria |
| `runtime.stackSize` | `"8MB"` | Tamaño máximo del stack |
| `gc.enabled` | bool | Habilitar GC |
| `gc.strategy` | `"active"` / `"compiled"` | Estrategia de GC |
| `sandbox.allowFS` | bool | Permitir acceso a FS |
| `sandbox.allowNet` | bool | Permitir acceso a red |
| `sandbox.maxExecutionTime` | ms | Timeout de ejecución |

---

## Artefactos de salida

| `target` | Archivo | Descripción |
|----------|---------|-------------|
| `executable` | `.clsapp` (zip) | App completa: WASM + manifiesto + assets |
| `library` | `.clslib` (zip) | Librería CLS: WASM + manifiesto |
| `dynamic-lib` | `.dll` / `.so` / `.dylib` | Librería dinámica nativa (futuro) |

### Estructura de .clsapp

```
app.clsapp (zip)
├── manifest.json           # Metadatos
├── module.clsconfig        # Config del compilador
├── main.wasm               # Código compilado
├── lib/                    # WASM modules adicionales
└── assets/                 # Recursos estáticos
```
