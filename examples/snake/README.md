# `snake` — Snake: lógica pura + replays (prueba #9)

Implementa la **lógica pura** de Snake en CLS (`update(state, input) -> state`)
de forma **determinista** (la comida usa un PRNG LCG que forma parte del estado)
y la valida reproduciendo **replays** de entrada. Sin FFI de terminal:
headless y cross-platform.

## Uso

```powershell
cargo build --bin clx
powershell -File examples\snake\replay.ps1
# un replay suelto:
target\debug\clx.exe run examples\snake\src\main.clsx -- examples\snake\tests\sweep.replay
```

## Formato de replay

Una línea: `W H SEED MOVES`

- `W H` — grilla.
- `SEED` — semilla del LCG de comida.
- `MOVES` — secuencia de `U`/`D`/`L`/`R`/`.` (el reverso se ignora).

Ej.: `12 8 1 RRRRRRRRRR` (choca contra la pared derecha).

## Estado final (canónico)

```
score=N alive=true|false dir=U|D|L|R len=N head=x,y food=x,y body=x,y;x,y;...
```

## Estructura

```
snake/
├── cls.json
├── src/main.clsx      # clase Snake (update puro) + parseo de replay + main
├── tests/*.replay     # grabaciones de input
├── tests/*.final      # estado final esperado (versionado)
├── tests/oracle.py    # oráculo Python (mismo algoritmo)
└── replay.ps1         # CLS vs .final (+ oráculo Python)
```

## Qué prueba (del lenguaje)

- **Clases con métodos** y estado mutable (`me.body`, `me.seed`, ...).
- **Arrays** `int[]` (cuerpo con coordenadas intercaladas), `while`, `%`, `&&`.
- **PRNG determinista** (LCG) → sin azar externo.
- Parseo de archivo (`fs.readFile`) y de texto char a char.

## Casos cubiertos

- **Colisión con pared** (`wall`, `sweep`).
- **Reverso ignorado** y **auto-colisión** (`reverse`, `loop`).
- **Crecimiento** por comer (`sweep`: score>0, len>3).
- **Game-over estable**: tras morir el estado no cambia con más input.

## Oráculo

1. **`.final` versionado** — estado final esperado (regresión).
2. **`tests/oracle.py`** — reimplementación del mismo algoritmo en Python
   (cross-check independiente del runtime CLS).

## Resultado

`replay.ps1` termina en `RESULTADO: OK` cuando cada replay reproduce el `.final`
(y el oráculo Python coincide).

## Nota

El render en terminal (raw-mode, `termios`/winapi) queda **fuera** de esta prueba
por portabilidad; el objetivo es la lógica determinista y su verificación.
