# Uso de CLS

CLS se ejecuta desde terminal con el comando `clx`.

## Comandos principales

```bash
ccls run <archivo> [args...]     # Ejecutar un script
ccls check <archivo>             # Verificar tipos
ccls build <archivo> -o <salida> # Compilar (pendiente)
ccls ast <archivo> --json        # Mostrar AST como JSON
```

## Primer script

Crea un archivo `hello.clsx`:

```ccls
function main(args: String[]) -> int {
    print("Hello, World!");
    return 0;
}
```

Ejecuta:

```bash
ccls run hello.clsx
```

## Pasar argumentos

```bash
ccls run hello.clsx arg1 arg2 --flag
```

Dentro del script, `args` es un `String[]` con los argumentos.

## Configuración del módulo

Cada proyecto puede tener un archivo `module.clsconfig` en la raíz:

```json
{
  "name": "mi-app",
  "version": "1.0.0",
  "project": {
    "entry": "src/main.clsx",
    "target": "executable"
  }
}
```

## Configuración inline

También se puede configurar dentro del código:

```ccls
#!cls
#config(typecheck = true, typestrict = false)
```
