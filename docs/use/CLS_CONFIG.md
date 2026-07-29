# Configuración de proyectos CLS

Todo proyecto CLS se configura mediante un archivo `cls.json` en la raíz del proyecto.
Este archivo unifica metadatos, configuración del compilador, del intérprete y dependencias.

---

## Estructura completa

```json
{
  "name": "mi-app",
  "version": "0.1.0",
  "description": "Una app CLS",
  "authors": ["tu-nombre"],
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
      "async": false,
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

---

## Campos del manifiesto

### Metadatos

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `name` | string | — | Nombre del proyecto (obligatorio) |
| `version` | string | `"0.1.0"` | Versión semver |
| `description` | string | `""` | Descripción corta |
| `authors` | string[] | `[]` | Lista de autores |
| `license` | string | `"MIT"` | Licencia SPDX |
| `registry` | string | `"https://registry.cls-lang.org"` | URL del registry de paquetes |
| `entry` | string | `"src/main.clsx"` | Punto de entrada del proyecto |

### `project`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `sourceDir` | string | `"src"` | Directorio del código fuente |
| `outDir` | string | `"dist"` | Directorio de salida (build) |
| `target` | string | `"executable"` | Tipo: `"executable"`, `"library"`, `"dynamic-lib"` |

### `compiler`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `targetArchitecture` | string | `"wasm"` | Arquitectura destino: `"wasm"`, `"x86_64"`, `"arm64"`, `"bytecode"` |
| `optimizationLevel` | string | `"O2"` | Nivel de optimización: `"O0"`, `"O1"`, `"O2"`, `"O3"`, `"Os"` |
| `sourceMaps` | bool | `true` | Generar source maps para debugging |

#### `compiler.types`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `check` | bool | `true` | Habilitar chequeo de tipos (true = híbrido, false = dinámico) |
| `strict` | bool | `false` | Modo estricto |
| `noImplicitAny` | bool | `false` | Prohibir tipos no inferidos |
| `nullSafety` | bool | `true` | Evitar null pointer exceptions |

#### `compiler.features`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `async` | bool | `false` | Soporte async/await |
| `macros` | bool | `false` | Macros |
| `experimental` | bool | `false` | Features experimentales |

#### `compiler.warnings`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `unusedVariables` | string | `"warn"` | `"warn"`, `"error"`, `"off"` |
| `deadCode` | string | `"warn"` | `"warn"`, `"error"`, `"off"` |
| `treatWarningsAsErrors` | bool | `false` | Tratar warnings como errores |

### `interpreter`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `optimization` | bool | `false` | Optimizar en interpretación |
| `mode` | string | `"pure-ast"` | Modo: `"pure-ast"`, `"jit"` (futuro) |

#### `interpreter.runtime`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `memoryLimit` | string | `"512MB"` | Límite de memoria |
| `stackSize` | string | `"8MB"` | Tamaño de pila |

##### `interpreter.runtime.gc`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Activar GC |
| `strategy` | string | `"compiled"` | `"active"` (runtime) o `"compiled"` (quemado en WASM) |
| `threshold` | string | `"64MB"` | Umbral para activar GC |

#### `interpreter.sandbox`

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `allowFs` | bool | `false` | Permitir acceso a filesystem |
| `allowNet` | bool | `false` | Permitir acceso a red |
| `maxExecutionTime` | int | `5000` | Timeout máximo en ms |

---

## Dependencias

```json
{
  "dependencies": {
    "http-server": "^1.0.0",
    "json-utils": "~0.2.0"
  },
  "devDependencies": {
    "test-runner": "^0.5.0"
  }
}
```

- Las claves son nombres de paquetes del registry
- Los valores son versiones semver
- `clx add <paquete>` agrega a `dependencies`
- `clx add <paquete> --dev` agrega a `devDependencies`
- `clx install` descarga los paquetes a `modules/<paquete>/mod.clsx`

---

## cls.lock (lockfile)

Generado automáticamente por `clx install`. Bloquea las versiones exactas instaladas.

```json
{
  "lockfileVersion": 1,
  "registry": "https://registry.cls-lang.org",
  "packages": {
    "http-server": { "version": "1.0.0" },
    "json-utils": { "version": "0.2.0" }
  }
}
```

| Campo | Descripción |
|-------|-------------|
| `lockfileVersion` | Versión del formato (siempre 1) |
| `registry` | Registry usado en la instalación |
| `packages` | Mapa de paquete → versión exacta instalada |

---

## Ejemplos de uso

### Crear proyecto nuevo

```bash
clx new mi-app
# Genera: cls.json + src/main.clsx + .gitignore
```

### Ejecutar con entry del manifiesto

```bash
cd mi-app
clx run
# Usa "entry": "src/main.clsx" del cls.json
```

### Ejecutar otro archivo

```bash
clx run tests/otro.clsx
```

### Build con entry del manifiesto

```bash
clx build
# Empaqueta entry + genera manifest.json dentro del .clsapp con name/version
```

### Build manual

```bash
clx build src/main.clsx -o dist/app.clsapp
```

### Check de tipos

```bash
clx check                 # escanea todo el proyecto
clx check src/            # escanea directorio
clx check src/main.clsx   # archivo específico
```

### Agregar dependencia

```bash
clx add http-server
clx add test-runner --dev
clx install
```
