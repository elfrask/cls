# Reglas del proyecto CLS — Fase final de desarrollo

> Documento de trabajo. Define el contexto, las restricciones y la filosofía que deben guiar cualquier decisión de diseño, prueba o evaluación del lenguaje CLS durante su fase final de desarrollo.

---

## 1. Estado actual del lenguaje

### 1.1 Arquitectura de ejecución

- CLS es un lenguaje **compilado e interpretado con JIT**.
- El **tree-walker fue deprecado y posteriormente eliminado** del repositorio.
- El JIT es ahora el **único motor de ejecución**.
- Existe un backend WASM en desarrollo (`.clbin`), aún no funcional.

### 1.2 Consecuencia crítica

Al no existir tree-walker, **no hay un segundo motor interno** que sirva como referencia para detectar divergencias semánticas. La red de seguridad que ofrecía la comparación `tree-walker ↔ JIT` ya no está disponible.

Esto implica que **cualquier verificación debe apoyarse en oráculos externos** al lenguaje (ver sección 4).

---

## 2. Filosofía del lenguaje

### 2.1 Autocontención total

> **Todo debe ser desarrollado dentro del mismo lenguaje.**

Esto incluye:

- Estructuras de datos
- Algoritmos
- Protocolos
- **Incluso las llamadas al sistema** (syscalls)

El lenguaje no debe depender de un ecosistema externo para ser útil. Si algo falta, se construye dentro de CLS.

### 2.2 FFI a C: la única puerta de escape

> **El FFI a C es la única puerta de escape permitida.**

Se usa para llegar al kernel y a librerías nativas del sistema. Es legítimo y esperado. Ejemplos válidos:

- Llamar a `socket`, `bind`, `listen`, `accept` vía FFI
- Llamar a `sqlite3_open`, `sqlite3_step` vía FFI
- Llamar a `tcsetattr`, `ioctl` vía FFI
- Llamar a `open`, `read`, `write`, `close` vía FFI
- Llamar a `fork`, `execve`, `pipe`, `dup2` vía FFI
- Escribir wrappers en CLS puro sobre esas primitivas C
- Construir una stdlib entera en CLS apoyada en FFI

El FFI **alimenta la API del lenguaje**. Es la base sobre la que se construye la stdlib autocontenida.

### 2.3 El host no provee features

> **El host no es una fuente de features. Es solo un puente al kernel.**

El host (Rust: `clx`, `clxr`, nodos) debe limitarse a:

- Proveer **primitivos mínimos** (los intrínsecos ya existentes)
- Proveer **`print` e `input`** como IO básico
- Proveer el **puente FFI** para que CLS hable con C
- **Nada más**

### 2.4 Prohibición de `host_calls`

> **Está prohibido crear `host_calls` o APIs desde el host para alimentar carencias que el FFI puede resolver.**

No se puede:

- Crear `host_calls` nuevos
- Crear intrínsecos nuevos para tapar carencias
- Añadir funciones nativas del runtime que no sean estrictamente primitivas
- Alimentar el lenguaje desde el nodo (`clx`, `clxr`) en lugar de desde el propio CLS

La regla operativa:

> **Si algo se puede hacer con FFI, se hace con FFI. No se crea un `host_call` para ello.**

### 2.5 Por qué importa esta distinción

**Validez del lenguaje.** Si el lenguaje necesita que el host le añada features constantemente, entonces no es un lenguaje útil por sí solo. Es una capa sobre Rust.

**Dogfooding real.** Cada carencia que se resuelve con un `host_call` es una carencia que nunca se prueba resolver dentro del lenguaje. Cada carencia que se resuelve con FFI + código CLS es:

- Una prueba de que el FFI funciona
- Una prueba de que el lenguaje es expresivo
- Un ladrillo más de la stdlib autocontenida

**Portabilidad.** Si la stdlib depende de `host_calls`, el backend WASM futuro tendrá que reimplementarlos todos, y cada nodo tendrá que reimplementarlos. Si la stdlib depende de FFI + CLS puro, el código CLS viaja con el lenguaje y solo hay que portar el puente FFI.

### 2.6 Tabla: qué va dónde

| Necesidad | Dónde se resuelve | Permitido |
|-----------|-------------------|-----------|
| Abrir un archivo | FFI a `open`/`read`/`write` + wrapper en CLS | ✅ |
| Hacer un socket | FFI a `socket`/`connect` | ✅ |
| Parsear HTTP | CLS puro | ✅ |
| Hacer un shell | FFI a `fork`/`execve` + CLS puro | ✅ |
| Hashing SHA-256 | CLS puro (o FFI a OpenSSL) | ✅ |
| `fs.readFile` como función nativa | Viola la regla | ❌ |
| `http.get` como `host_call` | Viola la regla | ❌ |
| `json.parse` como intrínseco | Viola la regla | ❌ |
| `print` / `input` | Host (IO básico) | ✅ |
| `len`, `type`, `int`, `float`, `str` | Intrínsecos primitivos | ✅ |
| `now`, `sleep`, `exit` | Intrínsecos primitivos (o FFI) | ✅ |

La stdlib actual (`math`, `json`, `fs`, `http`) debe ser **reevaluada** bajo esta regla. Todo lo que sea implementable en CLS + FFI **debería migrarse fuera del host**.

### 2.7 Violaciones conocidas de la autocontención (deuda a pagar)

Hoy el propio lenguaje **viola** la regla de autocontención: expone features
como `host_calls`/intrínsecos que deberían vivir en CLS + FFI. Se reconocen como
**deuda**, no como diseño válido:

| Feature actual | Tipo actual | Debería ser | Estado |
|---|---|---|---|
| `fs.*` (`readFile`, `writeFile`, `exists`, `rm`, `mkdir`, `listDir`, `cwd`) | `host_call` del nodo | FFI `open`/`read`/`write`/`close`/`opendir` + wrapper CLS en `/modules` | Deuda |
| `http.get` / `http.post` | `host_call` del nodo | FFI `socket`/`connect`/`send`/`recv` + HTTP en CLS puro en `/modules` | Deuda |
| `json.parse` / `json.stringify` | intrínseco | CLS puro en `/modules` | Deuda |
| `strings.{indexOf,slice,split}` | intrínseco fusionado | métodos de `String` en CLS / `/modules` | Deuda |
| `Lib.load` | `host_call` (stub) | resolver de `.clslib` + FFI | Deuda |

Regla derivada: **cada vez que una prueba necesite una de estas capacidades, se
implementa en CLS + FFI dentro de `/modules`** (ver sección 11), en vez de usar
el `host_call`. Cuando un módulo CLS reemplaza a un `host_call`, la migración se
anota aquí y el `host_call` queda candidato a eliminación.

> Estas violaciones son el motivo de que la **suite de conformidad** (sección 4)
> tenga que rechazar `host_calls` no primitivos (sección 9.3).

---

## 3. Estado del ecosistema

### 3.1 Lo que existe

- **Funciones intrínsecas mínimas** para primitivos.
- **IO básico otorgado por el host**: únicamente `print` e `input`.
- **FFI a librerías nativas de C**.
- Acceso parcial al ecosistema de C a través del FFI.

### 3.2 Lo que NO existe

- **No hay ecosistema de librerías**.
- **No hay stdlib extendida** más allá de lo mínimo mencionado.
- **No hay paquetes de terceros**.
- **No hay dependencias externas** al propio lenguaje.

### 3.3 Consecuencia

Cualquier funcionalidad que se necesite debe **construirse desde cero dentro del propio lenguaje**, apoyándose únicamente en:

- Intrínsecos primitivos
- FFI a C
- `print` / `input`

---

## 4. Estrategia de verificación (sin tree-walker)

Dado que no existe un motor de referencia interno, la verificación debe apoyarse en **oráculos externos**.

### 4.1 Oráculo 1 — C vía FFI (el más fuerte)

Escribir el **mismo algoritmo dos veces dentro del mismo programa CLS**:

- Una versión en CLS puro.
- Una versión que llama a C vía FFI (compilada aparte).

Comparar resultados. C actúa como semántica externa e independiente.

```cls
function fib_cls(n: int) -> int { ... };
function fib_c(n: int) -> int { return __ffi_call("fib_c", n); };

for (i = 0; i < 40; i++) {
    assert(fib_cls(i) == fib_c(i), "divergencia en i=$i");
};
```

### 4.2 Oráculo 2 — Especificaciones y herramientas externas

| Dominio | Oráculo |
|---------|---------|
| DNS | RFC 1035, `dig`, `nslookup` |
| HTTP | RFC 7230, `curl`, `nginx` |
| SQLite | `libsqlite3`, CLI `sqlite3` |
| Brainfuck | Intérpretes de referencia en C |
| Shell | `/bin/sh`, `dash`, `bash` |
| Scheme/Lisp | `guile`, `chicken-scheme`, SBCL |
| Hashing | `sha256sum`, `gpg` |

### 4.3 Oráculo 3 — El propio JIT en dos modos

Si el JIT soporta niveles de optimización (`optimizationLevel` en `cls.json`, ej. `O0` vs `O2`), compilar el mismo programa en ambos modos y comparar salidas.

Esto detecta **bugs del optimizer**, no del JIT base.

```bash
clx build app.clsx -O0 -o app_o0.clsapp
clx build app.clsx -O2 -o app_o2.clsapp
./app_o0 > out_o0.txt
./app_o2 > out_o2.txt
diff out_o0.txt out_o2.txt   # si difieren → bug del optimizer
```

---

## 5. Reglas para proyectos de prueba

Todo proyecto destinado a poner a prueba CLS debe cumplir:

### 5.1 Reglas obligatorias

1. **Ser autocontenido.** No puede depender de librerías externas fuera de CLS + FFI a C.
2. **Construirse dentro del lenguaje.** Si falta una estructura, se implementa en CLS.
3. **Tener un oráculo externo.** Sin tree-walker, la verificación debe venir de fuera.
4. **Ser reproducible.** Cualquier resultado debe poder regenerarse.
5. **Exponer regresiones.** Debe fallar de forma clara cuando algo se rompe.
6. **No tocar el host.** Si el proyecto necesita un `host_call` nuevo, eso es un bug del lenguaje.

### 5.2 Reglas de diseño

7. **Usar FFI solo cuando sea imprescindible** (syscalls, libs nativas del sistema).
8. **Dogfooding prioritario.** El lenguaje debe poder construirse a sí mismo.
9. **Aislar features.** Cada proyecto debe estresar un eje concreto del lenguaje.
10. **Documentar el fallo.** Cuando algo se rompa, debe quedar registro de qué feature falló.
11. **Preferir comparación diferencial** sobre tests de valor esperado cuando sea posible.

### 5.3 Reglas de priorización

12. **Mejor ROI primero.** Tests baratos que cubren mucho (Brainfuck, fuzzer) antes que proyectos grandes.
13. **Infraestructura antes que casos de uso.** La suite de conformidad vive con el proyecto; los demás son clientes.
14. **Bloquear release** hasta que la suite diferencial vs C pase.

### 5.4 Regla de detección

> **Si un proyecto de prueba necesita un `host_call` nuevo, eso es un bug del lenguaje**, no una razón para añadirlo. La solución correcta es: FFI + código CLS.

---

## 6. Áreas conocidas como frágiles

Basado en el historial reciente del repositorio:

| Área | Evidencia | Prioridad |
|------|-----------|-----------|
| **Strings (semántica de bytes)** | Commit `fix(strings)!: semantica de bytes + metodos de String` | Alta |
| **FFI de structs** | Devlog `fix del FFI de structs + estado 36+24` | Alta |
| **JIT como intérprete principal** | Commit `refactor(clxr): migrar a JIT como interprete principal` | Alta |
| **Optimizer** | Fase 3 completada, sin walker de referencia | Alta |
| **CMX** | Feature nueva, poco probada | Media |
| **Async/await** | Sintaxis presente, ejecución real no verificada | Media |
| **Gestor de paquetes** | Fase A recién iniciada (`driver gix`) | Media |
| **VFS + sandbox** | `app://`, `user://`, `tmp://`, `res://` | Media |

---

## 7. Proyectos de prueba (resumen)

| # | Proyecto | Oráculo | Esfuerzo | Prioridad |
|---|----------|---------|----------|-----------|
| 1 | Suite de conformidad + fuzzer vs C | C vía FFI, `-O0` vs `-O2` | Alto | **Bloqueante** |
| 2 | Shell POSIX | `/bin/sh`, `dash` | Medio | Alta |
| 3 | Brainfuck + benchmark | BF en C | **Bajo** | **Alta** (mejor ROI) |
| 4 | Servidor HTTP | `curl`, `nginx` | Medio | Alta |
| 5 | SQLite ORM | `sqlite3` CLI + lib | Alto | Alta |
| 6 | DNS resolver + autoritativo | `dig` | Medio | Media |
| 7 | Motor de plantillas CMX | Golden files | Medio | Media |
| 8 | Gestor de paquetes | `sha256sum` | Alto | Media |
| 9 | Snake/Tetris terminal | Grabaciones | Bajo | Baja |
| 10 | Compilador Lisp/Scheme | `guile` | Alto | Baja |

---

## 8. Recomendación de arranque

Dado que el tree-walker fue eliminado recientemente:

1. **Antes de cualquier proyecto grande**, montar el **proyecto 3 (Brainfuck)** en una sesión corta.
   - Intérprete en CLS (~150 LOC)
   - Intérprete en C (~100 LOC) expuesto vía FFI
   - Correr `mandelbrot.bf` con ambos
   - Comparar output y tiempo
   - Si divergen → bug del JIT detectado antes de invertir en proyectos grandes

2. **En paralelo**, iniciar el **proyecto 1 (suite de conformidad)**.
   - Es infraestructura de calidad que acompaña todo el desarrollo
   - Los demás proyectos son clientes de esta suite, no sustitutos

---

## 9. Consecuencias para la fase final

Antes de declarar el lenguaje estable, hay que verificar:

1. **Cada función de la stdlib actual** tiene una versión en CLS + FFI que la reemplaza, o justificación de por qué es intrínseco primitivo.
2. **Cada proyecto de prueba** funciona sin tocar el host.
3. **La suite de conformidad** rechaza `host_calls` no primitivos.
4. **El backend WASM futuro** podrá reusar la stdlib CLS sin reimplementar features.

Si la stdlib no puede migrarse a CLS + FFI, entonces el lenguaje **no es autocontenido** y hay que arreglarlo antes del release.

---

## 10. Principios rectores (resumen ejecutivo)

> 1. **Sin tree-walker → oráculos externos obligatorios.**
> 2. **Autocontención total:** todo se construye dentro de CLS.
> 3. **FFI a C es la única puerta de escape permitida** para llegar al kernel y a libs nativas.
> 4. **El host no provee features.** Solo primitivos mínimos + puente FFI.
> 5. **Prohibido crear `host_calls`** para tapar carencias resolubles con FFI.
> 6. **Si un proyecto necesita un `host_call` nuevo → es un bug del lenguaje.**
> 7. **La stdlib vive en CLS, no en el host.**
> 8. **Dogfooding como norma.**
> 9. **Comparación diferencial > tests de valor esperado.**
> 10. **La suite de conformidad rechaza `host_calls` no primitivos.**

---

## 11. Módulos compartidos (`/modules`) y rutas de búsqueda

### 11.1 Carpeta `/modules` del repositorio

Durante la fase de pruebas se construyen módulos en CLS que se **reutilizan
entre proyectos** (bases de la stdlib autocontenida: `io`, `str`, `http`, `json`,
`sha256`, `net`, ...). Su casa es la carpeta **`/modules`** en la raíz de este
repositorio.

- Un módulo vive en `modules/<nombre>/` con su `cls.json` (sección `export` +
  `"entry"`) y, opcionalmente, un `mod.clsx`.
- Es **CLS puro apoyado solo en FFI + primitivos + `print`/`input`** (regla 2.2).
- **No** puede introducir `host_calls` nuevos.
- Se versiona en este repo y es la única dependencia compartida permitida entre
  las pruebas (no hay paquetes de terceros, sección 3.2).

> Un módulo de `/modules` que resuelve una carencia del lenguaje es **un ladrillo
> de la stdlib autocontenida** y, cuando reemplaza a un `host_call`, se anota en
> la tabla 2.7.

### 11.2 Dos formas de agregar rutas de búsqueda

Como `/modules` está fuera de cada proyecto (`examples/<prueba>/`), el resolver
necesita que se le indique dónde buscar. Se agregan **rutas adicionales** de dos
maneras equivalentes:

**a) Flag de CLI** — `clx run`:

```bash
clx run examples/brainfuck/src/main.clsx --add-path-module=modules
clx run examples/brainfuck/src/main.clsx --add-path-module=modules --add-path-module=libs
```

- Forma `--add-path-module=<path>` (y `--add-path-module <path>`).
- Repetible: cada aparición **agrega** una ruta de búsqueda.
- Aplica al `clx run` en curso.

**b) Campo de `cls.json`** — `pathModules`:

```json
{
  "name": "brainfuck",
  "entry": "src/main.clsx",
  "pathModules": ["../../modules"]
}
```

- `"pathModules": String[]` — mismo efecto que el flag, para el proyecto.
- Relativas a la raíz del proyecto (CWD). El orden declarado es el orden de
  búsqueda.
- El flag y el campo se **combinan** (primero el `cls.json`, luego el flag).

### 11.3 Dónde entra en el orden de resolución

Las rutas adicionales se insertan como un **buscador más** en la resolución de
imports por nombre (`import "x"`, `from "x" import ...`). Orden:

1. `{dir del archivo que importa}/{x}.clsx` (local)
2. `{proyecto}/modules/{x}/` (módulos del proyecto)
3. **rutas de `pathModules` + `--add-path-module`** → `{ruta}/{x}/` y `{ruta}/{x}.clsx`
4. `{cwd}/{x}.clsx` y `{cwd}/modules/{x}/`
5. global `~/.cls/modules/`

El buscador adicional respeta el aislamiento de módulo: si `{ruta}/{x}/cls.json`
tiene `export.entry`, se usa ese entry.

### 11.4 Reglas

15. **Todo módulo reutilizable va en `/modules`** y se referencia con
    `--add-path-module=modules` o `"pathModules": ["..."]`.
16. **Prohibido copiar/pegar** un módulo entre proyectos: se referencia.
17. Un módulo de `/modules` **no añade `host_calls`**; si le falta algo, lo
    resuelve con FFI (es un bug del lenguaje si no puede).
18. El uso de estas rutas **no exime** la regla 5.1.6 (no tocar el host).

---

*Documento vivo. Actualizar cuando cambie el estado del JIT, del FFI o de la stdlib.*