# Contribuir

## Workflow

1. Haz un fork y crea una rama: `feature/mi-feature`.
2. Implementa el cambio siguiendo las convenciones.
3. Asegúrate de que compile sin warnings: `cargo build --workspace`.
4. Ejecuta los tests: `cargo test --workspace`.
5. Agrega tests unitarios para lo que cambiaste (ver `desarrollo/testing.md`).
6. Verifica los ejemplos: ejecuta los archivos de `examples/tests/`.
7. Crea un pull request con una descripción del cambio.

## Mensajes de commit

Usa `conventional commits`:

- `feat:` — nueva feature.
- `fix:` — corrección de bug.
- `refactor:` — cambio de estructura sin cambiar comportamiento.
- `docs:` — documentación.
- `test:` — tests.
- `chore:` — mantenimiento.

## Estilo de código

- `rustfmt` por defecto (edición 2021).
- Identación de 4 espacios.
- Los identificadores y strings del lenguaje usan convención de CLS:
  `snake_case` para funciones/variables, `PascalCase` para tipos.
- **No agregues comentarios innecesarios** en el código; los comentarios
  explican el "por qué", no el "qué".
- Evita dependencias nuevas si se puede resolver con la stdlib.

## Convenciones del proyecto

- Los mensajes de error y warnings del lenguaje van en **español** (es el
  idioma del proyecto).
- La documentación de una feature nueva va en `docs/` y se referencia desde el
  `README.md` de `docs/`.
- Los type maps (`.type.json`), los `.clsi` y el grammar de la extensión de VS
  Code se mantienen sincronizados con el lenguaje.
- El `agent-context/` contiene los planes de features (referencia interna).

## Reglas de arquitectura (importantes)

- **El core y el runtime son agnósticos al entorno.** No accedan al sistema de
  archivos, red ni a módulos de nodo (`fs`, `http`, `Lib`).
- **El nodo provee los resolvers** y los internals del nodo. El runtime solo
  ejecuta y carga módulos.
- **Los errores se formatean en `error_report.rs`** y los colores en
  `cls_core::ansi`. No dupliques formateo en los nodos.
- **Los errores de runtime deben mostrar siempre el trazo completo**; el
  typechecker se limita a un solo nivel (ver `runtime/errores.md`).

## Cómo reportar un bug

Incluye:

- El código mínimo que reproduce el problema.
- El comando y la salida (recorta el trazo de error completo).
- La versión (`clx --help` no muestra versión; revisa `Cargo.toml`).

## Pruebas manuales

Los ejemplos de `examples/tests/` cubren features individuales:

```
clx run examples/tests/test-methods.clsx
clx check --strict examples/tests/test-types.clsx
```

Los tests de importación se ejecutan desde `examples/tests/` (los imports son
relativos al directorio de trabajo).
