# Errores y control de errores

## Lanzar errores: `throw`

```clx
throw("error interno");
```

- `throw(msg)` lanza un error de runtime con el mensaje dado.
- Lanzado fuera de un `try`, termina el programa mostrando el trace completo.

## Captura: `try / catch / finally`

```clx
function lanzar(msg: String) { throw(msg); };
function nivel2() { lanzar("error interno"); };
function nivel1() { nivel2(); };

function main(args: String[]) -> int {
    try {
        print("dentro de try");
        nivel1();
        print("NO deberia imprimirse");
    } catch (e) {
        print("catch:", e);
    } finally {
        print("finally ejecutado");
    }

    try {
        var a = [1, 2, 3];
        print("indice fuera:", a[10]);    # error de índice capturado
    } catch (e2) {
        print("catch indice:", e2);
    }
    print("tras los try");
    return 0;
};
```

- El parámetro del `catch` es un **String** con el mensaje limpio
  (`"Error de runtime: ..."`).
- Se permiten **múltiples cláusulas `catch`** consecutivas (con tipo opcional
  tras `:`), seguidas de un único `finally` opcional.
- El call stack CLS se conserva al capturar; `execute_try` restaura la
  profundidad previa.

Ejemplo completo: `examples/audit/features/15-try-catch.clsx`.

## Errores no capturados

Un error sin capturar se propaga hasta el CLI, que muestra **siempre el trace
completo**: import_trace + call stack numerado, con el código fuente por frame
y el caret del frame del error (ver `runtime/errores.md`). Está
prohibido mostrar solo el mensaje.

## En el JIT

- **wasmtime** (default): el backend emite una sección de excepciones WASM
  (tag + `try_table`). `throw` y errores de runtime llevan payload
  `(mensaje, span empaquetado)` que el host desempaca para renderizar el caret
  exacto.
- **wasmi** (`CLS_JIT_RUNTIME=wasmi`): intérprete puro **sin soporte de
  excepciones** — `try`/`catch`/`throw` fallan en compilación y los errores
  son traps con el mensaje, sin caret.
- Los errores de tipo del typeck estricto abortan antes de emitir.
