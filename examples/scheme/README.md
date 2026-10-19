# `scheme` — Intérprete Scheme en CLS puro (prueba #10)

Implementa un **intérprete Scheme (subconjunto)** en CLS puro: reader de
S-expresiones + evaluador con entornos, closures y recursión. Ejercita a fondo
el compilador (parser, clases, funciones como valor, recursión, arrays, strings).

## Uso

```powershell
cargo build --bin clx
powershell -File examples\scheme\diff.ps1
# o un programa suelto:
target\debug\clx.exe run examples\scheme\src\main.clsx -- examples\scheme\tests\lists.scm
```

## Estructura

```
scheme/
├── cls.json
├── src/main.clsx      # Val + Reader + Interp + main (un archivo)
├── tests/*.scm        # programas Scheme
├── tests/*.out        # salida esperada (oráculo versionado)
└── diff.ps1           # corre cada .scm y compara con .out (+ guile opcional)
```

## Representación

Todos los valores se modelan con una única clase `Val` (`kind` + campos): `num`,
`sym`, `str`, `bool`, `nil`, `pair`, `prim`, `closure`, `env`, `err`. Como `null`
no es asignable a una clase, la lista vacía es el centinela `kind == "nil"`.
El reader y el evaluador son clases (`Reader`, `Interp`) para permitir recursión
mutua entre métodos (las funciones libres deben definirse antes de usarse).

## Subconjunto soportado

- **Formas especiales**: `quote`, `if`, `cond` (con `else`), `define` (variables
  y `(define (f args) ...)`), `lambda`, `let`, `begin`, `set!`, `and`, `or`.
- **Primitivas**: `+ - * / modulo abs`, `= < > <= >= max min`, `car cdr cons
  list null? pair? number? symbol? string? procedure? not eq? length append`,
  `display write newline`.
- **Closures** con captura de entorno, **funciones de orden superior**
  (`map`/`compose` definidos en Scheme), **recursión** (factorial, fib,
  Ackermann).
- Reader con comentarios `;`, strings con escapes `\n`/`\t`, `'` como quote.
- Números enteros (sin floats).

## Limitaciones (documentadas)

- Sin **tail-call optimization** (recursión profunda limitada por la pila).
- Sin `quasiquote`, `let*`/`letrec`, vectores, floats, `set-car!`, `apply`,
  continuaciones, ni GC explícito (la memoria la gestiona el runtime CLS).
- El objetivo es **probar el compilador CLS** con un workload real, no ser un
  Scheme completo.

## Oráculo

1. **Salidas esperadas versionadas** (`tests/*.out`) para semántica estándar
   (factorial, fib, Ackermann(3,3)=61, listas, closures, strings).
2. **`guile`** (opcional, si está en el PATH): cross-check de los mismos
   programas contra una implementación Scheme formal.

## Resultado

`diff.ps1` termina en `RESULTADO: OK` cuando los 7 programas coinciden con la
salida esperada (y, si hay `guile`, también con guile).
