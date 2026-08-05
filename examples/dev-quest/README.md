# dev-quest — camino al mejor dev del mundo

Un CLI gamificado en CLS: conviertes tu aprendizaje en misiones, ganas XP,
subes de nivel y construyes tu racha diaria. Hecho por un semi-senior que
antes era un junior entusiasta... y que aprendió a modularizar.

## Cómo correr

Desde esta carpeta:

```powershell
# Modo interactivo (menú)
clx run main.clsx

# Modo comando directo
clx run main.clsx -- tarea add "Aprender CLS"
```

## Comandos

```
tarea add <título> [categoria]   Añade una misión (Aprendizaje|Practica|Proyecto|Otro)
tarea list [estado]              Lista tus misiones (filtro: Pendiente|EnProgreso|Completada)
tarea prog <id>                  Pasa una misión a 'En progreso'
tarea done <id>                  Completa una misión y gana XP
habilidad add <nombre>           Registra una habilidad a dominar
habilidad list                   Tus habilidades
habilidad practica <id>          Practica (+5 XP, subes nivel cada 3 prácticas)
log add <mensaje>                Escribe en tu diario
log list                         Tu diario
stats                            Tu progreso, nivel y racha
frase                            Una dosis de motivación (con fallback local)
reset                            Borra todo el progreso
help / version                   Ayuda o versión
```

## Niveles

Novato (0) < Aprendiz (50) < Junior (150) < Senior (350) < Mago (700)

## Arquitectura

```
cls.json            Manifiesto del proyecto (strict mode activo)
main.clsx           Orquestador: comandos, persistencia (fs/json) e interfaz
modelo.clsx         Dominio puro: enums + clases (Tarea, Habilidad, EstadoJuego)
estadisticas.clsx   Lógica pura: niveles, porcentajes y racha
frases.clsx         Frases motivacionales con fallback local
data/dev-quest.json Progreso guardado (se crea solo al jugar)
```

### ADR: por qué el modelo es un módulo y main sigue siendo grande

Separación de responsabilidades:

- **`modelo.clsx`** — el dominio, aislado y reutilizable. Exporta sus enums y
  clases con `export`; `main` los construye con `modelo.Tarea(...)` y
  `modelo.EstadoTarea.Pendiente`.
- **`estadisticas.clsx` / `frases.clsx`** — lógica pura sin efectos.
- **`main.clsx`** — lo que depende del entorno: `print`/`input` (comandos e
  interfaz), `fs`/`http` (persistencia y frases remotas), `now`/`len`/`int`.

`main` no es pequeño a propósito: los módulos de CLS **no ven los intrinsics
del runtime** (`print`, `input`, `len`, `int`, `now`...) ni los internos del
nodo (`fs`, `http`), así que todo lo que los necesita vive en el archivo de
entrada. Modularizar la capa de comandos/persistencia tendría que esperar a
que el runtime permita intrinsics en módulos (o inyectarlos como parámetros).

Decisiones concretas:

- **Clases planas, sin herencia**: el runtime aún no resuelve `super` entre
  módulos ("La clase X no tiene clase padre"). En vez de una base `Entidad`,
  cada clase declara su `id`. Si el runtime lo soporta, se puede reintroducir
  la herencia sin tocar a los llamadores.
- **Sin enum en el cuerpo de métodos de módulo**: los métodos de clases
  exportadas no resuelven los enums del módulo por nombre; los *defaults* de
  campo sí. Por eso `Tarea` nace con `estado = Pendiente` vía su valor por
  defecto, no en el constructor.
- **Módulos hoja**: `main` importa módulos que solo dependen de core
  (`math`/`json`). Los módulos que importan otros módulos de usuario aún no
  resuelven en el runtime.

### Lo que aprendí por las malas (y con cariño)

- CLS pasa las instancias **por valor** a las funciones: mutar un parámetro no
  cambia el objeto del llamador. Los comandos **devuelven el estado** nuevo y
  `main` lo reasigna.
- Los mutadores de array (`push`, `pop`) solo escriben de vuelta cuando el
  receptor es una variable simple. Con miembros (`estado.tareas.push(x)`) la
  mutación se pierde. Patrón salvador: leer a un buffer tipado, mutarlo y
  asignarlo de vuelta. Mutar un elemento de array, igual.
- `clx run <archivo> -- <args>`: los argumentos van después de `--`.

## Tipado

El proyecto corre en modo estricto (`compiler.types.strict: true`). Cada
función declara sus tipos y el checker (`clx check --strict .`) es tu red de
seguridad. ¡Pásale con frecuencia!
