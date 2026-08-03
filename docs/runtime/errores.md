# Sistema de errores

## Tipos de error

Los errores se representan con `ClsError` (`cls-core/src/error/mod.rs`):

- `SyntaxError(String)` — error de sintaxis legacy (span incrustado en el texto).
- `SyntaxErrorAt(String, Span)` — error de sintaxis con **span estructurado**.
- `RuntimeError(String)` — error en ejecución.
- `TypeError(String)` — error de tipos.
- `CompileError(String)` — error de compilación.
- `IoError(std::io::Error)` — error de entrada/salida.
- `ConfigError(String)` — error de configuración.

## Fábrica de errores de sintaxis

Los errores de sintaxis se construyen con la fábrica centralizada:

```
ClsError::syntax_at("mensaje", &span)   // → SyntaxErrorAt
```

El parser y el lexer usan el helper `self.syntax_err("mensaje")`, que crea un
`SyntaxErrorAt` con la posición del token actual. El mensaje queda limpio; la
ubicación vive en el `Span`.

`extract_line_col` es solo un fallback para los errores legacy que incrustan
`(línea N, columna M)` en el texto (por ejemplo, algunos `RuntimeError`).

## Formateo centralizado

El formateo vive en `cls-runtime/src/error_report.rs`. El runtime produce un
`ErrorReport` (error + span + call stack + import trace + archivo) y lo
convierte a texto con el formato que el nodo elija.

### Formatos

```
enum ErrorFormat {
    Plain,     // texto plano sin decoradores
    Console,   // texto con códigos ANSI (colores)
    Html,      // texto HTML
    Json,      // JSON estructurado
}
```

El sistema es extensible: para añadir un formato, implementa el trait
`ErrorFormatter` y agrégalo a `format_error`.

### API

```
format_error(&report, &format) -> String      // el corazón
format_runtime_error(&report, &format)        // wrapper para runtime
format_syntax_error(&error, &source, &file, &format)
```

Los wrappers `show_runtime_error` / `show_syntax_error` imprimen por `stderr` en
formato `Console` (compatibilidad). El nodo puede usar `format_error` para pedir
otro formato y decidir dónde imprimir.

### Colores

Los códigos ANSI están centralizados en `cls_core::ansi` (`fg`, `bold`, y las
constantes de color). El `ConsoleFormatter` los usa; ningún otro módulo define
colores propios.

## Regla obligatoria

- **Typechecker** (`clx check`): los errores se limitan a fallos de un solo
  nivel (el archivo verificado). Muestra `archivo:línea:columna` + contexto de
  código (línea fuente + caret), sin trace de imports.
- **Runtime/compilación** (`clx run`, `clxr`, build): debe mostrar SIEMPRE el
  trazo completo — import trace + call stack numerado con código fuente por
  frame + el frame del error con caret. No es opcional: está prohibido mostrar
  solo el mensaje.

## Trazo de runtime

El intérprete conserva `call_stack` (pila de llamadas) y `import_trace`
(imports en curso). Al ocurrir un error, `build_error_report` construye el
reporte y `format_error` produce:

```
Error de ejecución:

1. → main (main.clsx)
2. En main.clsx:10:20 → outer
  10 |     return inner(y);
     |                    ^
3. En main.clsx:2:17 [Runtime Error]
  2 |     return x / 0;
    |                 ^
  Error: División por cero
```

El call stack no se "popea" al fallar (para conservarlo en el reporte); un
`try/catch` restaura la profundidad al capturar el error.

## En `clx check`

Los diagnostics del verificador muestran contexto de código con colores:

```
[ERROR] Operador + no soportado entre String y Int (2:38)
  2 |     return "Hello, " + name + "!" + 2;
    |                                      ^
```

- `[ERROR]` rojo, `[WARN]` amarillo, éxito en verde.
- La ubicación en gris y el caret en el color de la severidad.
