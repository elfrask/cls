# GC y gestión de memoria en WASM

Documento de diseño para la gestión de memoria del backend WASM (`.clbin`).
Complementa `WASM_PIPELINE.md` (cómo se emite el binario) y `JIT_RUNTIME.md`
(cómo `clxr` lo ejecuta). Aquí se define **dónde viven los objetos, quién los
asigna y quién los libera**.

## 1. El problema

WebAssembly usa **memoria lineal**: un único buffer de bytes direccionable por
enteros de 32 bits (`memory[ptr]`). No existe un heap gestionado por el runtime
como el de Rust o Java. Todo lo que no sea un valor plano (int, float, bool)
vive en esa memoria lineal, y el programa (o el runtime host) es responsable de:

1. Asignar regiones de memoria lineal (allocator).
2. Inicializarlas (layout de los objetos CLS).
3. Liberarlas cuando ya no se usan (free / GC).
4. Evitar fugas, dobles liberaciones y punteros colgantes.

Además, WASM no tiene excepciones nativas ni `try/finally` del host: la
liberación de recursos debe ser explícita o delegada a un GC.

## 2. Qué se compila a memoria

Cada tipo CLS tiene un layout en la memoria lineal:

| CLS | Layout en memoria lineal |
|-----|--------------------------|
| `int` | `i64` plano (en stack/registro, no en memoria) |
| `float` | `f64` plano |
| `bool` | `i32` (0/1) plano |
| `char` | `i32` (punto de código) plano |
| `String` | `(ptr: i32, len: i32)` → bytes UTF-8 en memoria |
| `Array<T>` | `(ptr: i32, len: i32, cap: i32, stride)` → elementos contiguos |
| `Tuple<...>` | struct plano (offsets fijos, sin puntero extra) |
| `Record<K,V>` | tabla hash (entradas en memoria) o linear scan |
| `structure` | struct plano (campos embebidos); campos complejos → punteros |
| `class Object` | bloque de campos + vtable pointer |
| `enum` | `u8`/`u16` (índice) — sin asignación de heap |
| `closure` | bloque de capturas (nun-boxed) |
| `CmxValue` | nodo de árbol (tag + props + children) |

Los primitivos planos **no tocan el heap**. Los contenedores (`String`,
`Array`, `Record`, `Object`, `closure`, `CmxValue`) sí requieren memoria.

## 3. Estrategia de asignación

### 3.1 Arena / region (recomendada para el primer backend)

Una **arena** (o *region*) es un bloque grande de memoria lineal que el bump
allocator reparte hacia adelante:

```
[ arena base ][  objeto1  ][  objeto2  ][ ... ][ bump → ][    libre    ][ arena fin ]
```

- **Asignación**: un `bump pointer` avanza; O(1), sin free individual.
- **Liberación**: por lotes — al final de una región (una llamada, un frame,
  una tarea) el bump pointer retrocede y todo lo de la región se descarta.
- **Ventajas**: simple, rápido, sin fragmentación, sin collector.
- **Costo**: los objetos que deben sobrevivir a la región (por ejemplo, un
  `String` retornado de una función que sale de un closure) hay que **copiarlos
  o promoverlos** a otra región.

Casos de uso:

- Región por **llamada de función** (los temporales mueren al retornar).
- Región por **marco de ejecución** de una corrutina.
- Región global para datos de larga vida.

### 3.2 Allocator simple (para objetos de larga vida)

Cuando un objeto debe sobrevivir sin un límite claro (registros, clases,
estructuras que viven toda la app), se usa un allocator general de bloques
(estilo `dlmalloc`/`wee_alloc`/`bumpalo` como punto de partida):

- Bloques con cabecera `{ size, next, free }`.
- Free list (best-fit / first-fit).
- `free(ptr)` explícito o por el GC.

### 3.3 Pools de tamaño fijo

Para objetos homogéneos frecuentes (por ejemplo, nodos `CmxValue`, celdas de
records pequeños), un **pool** de slots de tamaño fijo reduce fragmentación y
acelera la asignación.

### 3.4 Decisión recomendada

Un **híbrido**: arena (hot path, temporales) + allocator de bloques (objetos
persistentes). El GC, si está activo, recorre el allocator de bloques; la arena
no necesita GC (se libera por lotes).

## 4. Estrategias de GC

El manifiesto `cls.json` ya expone la configuración de GC:

```json
"interpreter": {
  "runtime": {
    "gc": {
      "enabled": false,
      "strategy": "compiled",
      "threshold": "64MB"
    }
  }
}
```

### 4.1 `--no-gc` + arena (modo embebido / alto rendimiento)

- **Sin collector**. Solo `structure`, `Tuple` y tipos planos en el hot path.
- Los punteros de campos complejos apuntan a la región; la región se libera en
  lote.
- Memoria mínima, layout plano, sin pausas.
- Restricción: no se permiten ciclos ni objetos de vida ilimitada fuera de la
  región.

### 4.2 `--gc runtime` (GC preciso, tracking)

Un GC *tracing* (mark-sweep o mark-compact) sobre el allocator de bloques:

- **Raíces (roots)**: la pila de llamadas, `self_stack` (`me`/`super`), los
  scopes del entorno, los registros de clases y los frames de corrutinas.
- **Marcado**: recorre las raíces; cada objeto visitado se marca (bits en la
  cabecera o un bitmap externo).
- **Barrido**: libera bloques no marcados.
- **Pausas**: el mark-sweep simple detiene el mundo. Un *incremental* o
  *generacional* reduce las pausas (ver 4.4).

### 4.3 GC generacional (futuro)

- **Nursery** (jóvenes): arena pequeña; los sobrevivientes se promueven.
- **Old gen**: mark-sweep/mark-compact con menos frecuencia.
- La mayoría de los objetos CLS son de corta vida (temporales de funciones), por
  lo que un generacional es muy efectivo.

### 4.4 Híbrido con la arena

La arena no requiere GC: los temporales se liberan en lote. El GC solo recorre
el allocator de bloques (objetos que escaparon). Esto combina la velocidad de la
arena con la seguridad del collector.

### 4.5 `threshold` — cuándo disparar el GC

- `threshold` (por defecto `64MB`) es el volumen de memoria asignado desde el
  último GC. Al superarlo, se dispara.
- Configurable por proyecto.
- Con arena, el "GC" es en realidad el reset del bump pointer al final de cada
  región (no depende del threshold).

### 4.6 Estrategia `compiled` (quemada en WASM)

El GC se compila en el binario como parte del runtime embebido (código WASM),
no como host. Ventaja: el `.clbin` es autónomo. Desventaja: más código en el
binario. Es la estrategia por defecto.

## 5. Referencias vs valores y ciclos

- **Semántica de valor**: CLS copia valores al asignar (los primitivos y
  `Tuple` son planos). En WASM, copiar un `Tuple` plano es una copia de memoria
  (o pasarlo por valor en registros).
- **Write-back de arrays**: los arrays mutables se escriben de vuelta a la
  variable. En WASM, un array es `(ptr, len, cap)`; mutar elementos es escribir
  en la memoria lineal; el write-back es trivial (no hay copia).
- **Ciclos**: un objeto que referencia a otro (incluso indirectamente) crea un
  ciclo que el `--no-gc` no puede liberar. El GC *tracing* los maneja; el modo
  arena requiere que el usuario evite ciclos persistentes.

## 6. Punteros y representación

- **Punteros a memoria lineal**: offsets `i32` (WASM32). Un `null` se representa
  como `0` (offset base no usado).
- **Tagging de valores dinámicos**: para el slow path (`Any`, records, objetos)
  un valor es un entero etiquetado: `(tipo << 32) | ptr` o un par
  `(tipo, ptr)`:
  - Tipo bajo (p. ej. 4 bits): int/float/bool/char planos se empaquetan en el
    mismo entero (NaN-boxing o pointer-tagging).
  - Los objetos son `ptr` a memoria lineal.
- **vtable pointer**: cada clase lleva un puntero a su tabla de métodos (el
  "despacho de magic methods" resuelto en compile-time apunta a funciones
  concretas; la vtable cubre los métodos dinámicos que no se monomorfizaron).

## 7. Interacción con el intérprete actual

El intérprete actual (`cls-runtime`) usa `Value` (enum Rust) con `Box`,
`String`, `HashMap` y `Arc<Mutex<Environment>>`. Esto NO se transpila a WASM
directamente: es la representación de *desarrollo*.

Para el backend WASM:

1. El **typechecker** produce el AST tipado (qué tipo tiene cada expresión).
2. El backend **baja** ese AST a IR lineal y emite WASM con los layouts de la
   sección 2.
3. El **runtime embebido** (allocator + arena + GC) se compila junto al binario
   o se provee como import del host.

El `GarbageCollector` stub (`cls-runtime/src/gc.rs`) documenta la API que el
runtime WASM debe implementar: `collect()`, tracking de raíces, etc.

## 8. Raíces del GC (lista de control)

El GC debe conocer todas las referencias desde fuera del heap:

| Raíz | Descripción |
|------|-------------|
| Pila de llamadas | frames de funciones (variables locales y parámetros) |
| `self_stack` | `me` / `super` durante llamadas a métodos |
| Scopes del entorno | variables globales y de módulo |
| Registros de clases/structs/enums | definiciones persistentes |
| Frames de corrutinas | estado de las `Promise`/corrutinas en vuelo |
| Argumentos de funciones nativas | mientras se ejecutan |

El compilador emite **mapas de raíces** por función (qué slots son punteros),
para que el GC los marque sin recorrer toda la memoria (GC preciso).

## 9. Plan de implementación por fases

### Fase M1 — Memoria base
- Integrar un allocator simple sobre la memoria lineal WASM.
- Layouts de `String` y `Array<T>`.
- Región/arena por llamada de función para temporales.

### Fase M2 — Estructuras y tuplas planas
- `structure` y `Tuple` con offsets fijos (sin punteros salvo campos complejos).
- Habilitar `--no-gc` para el subconjunto estático.

### Fase M3 — GC por conteo de referencias
- Objetos con contador; liberación cuando llega a cero.
- Simple, determinista; no maneja ciclos (documentado).

### Fase M4 — GC mark-sweep preciso
- Bitmap de raíces por función.
- Marcado desde las raíces de la sección 8.
- Barrido del allocator de bloques.
- Disparo por `threshold`.

### Fase M5 — GC generacional (futuro)
- Nursery + old gen.
- Promoción de sobrevivientes.
- Pausas menores/mayores.

### Fase M6 — Arena + GC híbrido
- Arena para temporales (sin GC) + mark-sweep para objetos persistentes.
- Objetivo: rendimiento cercano a `--no-gc` con seguridad de collector.

## 10. Tabla de decisiones

| Aspecto | Decisión |
|---------|----------|
| Memoria | Memoria lineal WASM (WASM32). |
| Primitivos | `i64`/`f64`/`i32` planos, sin heap. |
| Temporales | Arena/region por llamada. |
| Objetos persistentes | Allocator de bloques (estilo dlmalloc). |
| `--no-gc` | Arena + liberación por lotes; embebidos/hot path. |
| `--gc runtime` | Mark-sweep preciso (luego generacional). |
| Raíces | Mapas de raíces por función (GC preciso). |
| Ciclos | GC tracing los maneja; arena requiere evitarlos. |
| Punteros | Offsets `i32`; `null` = `0`. |
| Tagging | NaN-boxing / pointer-tagging para valores dinámicos. |
| `threshold` | Volumen asignado que dispara el GC (por defecto 64MB). |
| Config | `cls.json` → `interpreter.runtime.gc`. |

## 11. Referencias

- `docs/future/wasm/WASM_PIPELINE.md` — cómo se emite el binario.
- `docs/future/wasm/JIT_RUNTIME.md` — cómo `clxr` ejecuta `.clbin`.
- `docs/future/native/NATIVE_AOT.md` — el mismo modelo para LLVM (arena/region,
  layout plano, `--no-gc`).
- `cls-runtime/src/gc.rs` — el stub `GarbageCollector`.
- `cls-core/src/config/types.rs` — `GcConfig` y `RuntimeMemoryConfig`.
