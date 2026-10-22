# `check` — aserciones y protocolo `CHECK k=v`

Módulo **CLS puro** (sin FFI, sin `host_calls`) con aserciones para runners de
prueba. Emite una línea `CHECK <name>=...` por aserción (el mismo protocolo que
consumen los oráculos externos, p. ej. `examples/sqlite-orm`) y lleva la cuenta
de fallos en estado de módulo.

## API

| Símbolo | Descripción |
|---|---|
| `fail(name)` | registra un fallo (`CHECK name=FAIL`) |
| `assertEq(name, got, exp)` | igualdad de enteros |
| `assertEqStr(name, got, exp)` | igualdad de strings |
| `assertTrue(name, cond)` | verdadero/falso |
| `fails() -> int` | fallos acumulados desde el último `reset()` |
| `reset()` | reinicia el contador |

Formato emitido:

```
CHECK <name>=ok
CHECK <name>=FAIL
CHECK <name>=FAIL got=<got> exp=<exp>
CHECK <name>=FAIL got='<got>' exp='<exp>'
```

Uso:

```clx
import "check" as check;

check::reset();
check::assertEq("sum.2+2", 2 + 2, 4);
check::assertEqStr("greet", greet(), "hola");
check::assertTrue("is.even", n % 2 == 0);
if (check::fails() > 0) { return 1; }
```

## Consumir

```clx
import "check" as check;
```
