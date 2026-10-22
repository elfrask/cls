# `cmx-templates` — Motor de plantillas CMX + renderer CLS puro

Prueba #7 del [plan de pruebas](../PLAN-PRUEBAS.md). Ejercita **CMX** (el DOM
virtual nativo de CLS) y un **renderer CMX→HTML en CLS puro** (0 host nuevo),
al estilo de la especificación de `modules/view.clsx` (minilaravel F6).

## Oraculos

1. **Golden files** (`tests/golden/*.html`) — la salida esperada está
   versionada; cualquier cambio de CMX/runtime/engine rompe un golden. Sin
   tree-walker, el golden **es** el oráculo.
2. **Jinja2 (autoescape)** (`tests/oracle/render_jinja.py`) — cross-check
   independiente sobre el subconjunto de paridad: interpolación, bucles y
   escape de `& < > '` (Jinja2/markupsafe coincide; las comillas dobles difieren
   —CLS `&quot;` vs Jinja2 `&#34;`— y por eso quedan fuera del subset).

## Estructura

```
cmx-templates/
├── cls.json
├── run.ps1                    # suite: render + golden + Jinja2 + findings
├── src/
│   ├── main.clsx              # renderiza cada pagina a out/*.html
│   ├── engine.clsx            # e(), raw(), render(), attrs, void elements
│   ├── components.clsx        # layout, navbar, card, badge, todoList
│   └── pages.clsx             # vistas (una por eje)
├── tests/
│   ├── golden/*.html          # oraculo (versionado)
│   ├── oracle/render_jinja.py # oraculo cruzado Jinja2
│   └── findings/*.clsx        # repros de bugs (exit != 0 = bug presente)
└── out/                       # salida generada (gitignore: **/out/)
```

## Correr

```powershell
cargo build --bin clx
powershell -File examples\cmx-templates\run.ps1          # verifica
powershell -File examples\cmx-templates\run.ps1 -Update  # regenera goldens
```

Requiere `target\debug\clx.exe`. Jinja2 es opcional (`pip install jinja2`); si
no está, el paso 3 se marca `SKIP`.

## Motor (`src/engine.clsx`)

- `e(s)` — escape de `&`, `<`, `>`, `"`, `'` (una sola pasada; **no**
  doble-escape; byte-transparente con UTF-8 multibyte/emoji).
- `render(c)` — recorrido recursivo: `kind==1` es texto (se escapa); si no,
  elemento con `<tag attrs>children</tag>`; `raw` emite su contenido **sin**
  escapar; los tags vacíos (`br`, `img`, `input`, …) se auto-cierran.
- `attrs(c)` — props ordenadas alfabéticamente (salida determinista); un prop
  `true` se emite como atributo booleano desnudo; los valores se escapan.
- `raw(html)` — marcador de HTML confiable (no escapa).
- `renderEach(items, fn)` — helper de bucles: renderiza y concatena.

## Qué cubre (goldens)

| Página | Eje |
|---|---|
| `01-escape` | `< > & " '` + UTF-8 multibyte (emoji) |
| `02-interpolation` | `{expr}`, aritmética, ternario |
| `03-attributes` | attrs string/int/bool, orden alfabético |
| `04-void` | elementos vacíos + booleano `checked={true}` |
| `05-raw` | `raw()` sin escape vs texto escapado |
| `06-components` | componentes como funciones que devuelven `Cmx` |
| `07-list` | navbar + lista de tareas (bucles + escape) |
| `08-full` | layout + navbar + card + lista (composición) |
| `09-parity` | subset reproducible por Jinja2 |
| `10-component` | componente por tag mayúscula (`isComponent` + `tagCallable`) |
| `11-bare-attr` | atributo desnudo (`<input disabled />` → `true`) |
| `12-cmx-methods` | clase `CMX`: `isPrimitive/isEmpty/isElementOf/flattenChildren`, `is Cmx` |
| `13-cmx-debug` | `toString()` de CMX (representación de depuración, escapada) |
| `14-spread-shorthand` | spread `{...base}`, shorthand `{var}`, desnudo |
| `15-clone-with` | `cloneWith(props, children)` inmutable |
| `16-erased-generics` | `CMX<P, PP>` borrado + `is Cmx` |

## API de la clase `CMX` (cubierta)

El builtin `Cmx` se expone como la **clase core `CMX`** (métodos en CLS,
`cls-core/src/frontend/builtins.rs`). Campos: `tag` (String, siempre), `props`
(`Record<String, Value>`), `children` (`Cmx[]`), `kind` (0=elemento/1=texto),
`tagCallable` (`Value`), `isComponent` (`Bool`).

| Método | Resultado |
|---|---|
| `isPrimitive()` | `!isComponent` |
| `isEmpty()` | `tag.length == 0` |
| `isElementOf(tag)` | `me.tag == tag` |
| `flattenChildren()` | `Cmx[]` (mismo `children`) |
| `tryCallTag()` | invoca `tagCallable(props, children)` si es componente; si no, `null` |
| `toString()` / `__str` / `__repr` | representación de depuración (`<tag ...>... (n children)</tag>`) |
| `cloneWith(props?, children?)` | copia inmutable (conserva tag/kind/tagCallable/isComponent) |

## Limitaciones de CMX (documentadas)

1. **Sin `for`/`if` estructurales** dentro del marcado: los bucles se hacen con
   `render`/`renderEach` + `raw(...)`, o `{cond ? a : b}`.
2. **Los children que son `Array` no se expanden** (`{arr.map(...)}`): en CLS
   puro no hay forma de distinguir en runtime un `Cmx` de un `Array`
   (`is Array` es compile-time; `type()` es estático; `.kind` lee basura sobre
   un array). Por eso el motor resuelve bucles con `renderEach`+`raw`.
3. **Los componentes** (tag mayúscula) sí se auto-invocan: el nodo trae
   `isComponent=true` y `tagCallable`, y el renderer lo llama con
   props+children. Los componentes devuelven HTML (`String`).
4. **Escape por defecto**: todo lo interpolado se escapa salvo `raw()`.

## Findings (bugs hallados)

`run.ps1` los corre y espera `exit != 0` mientras existan (convención de
`examples/lang-features`): mientras haya findings abiertos, el resultado global
es `FAIL` (es señal, no una regresión de esta prueba). Al arreglarse cada bug,
se promueve a la matriz (goldens) o se marca GAP en `gaps.txt`.

| # | Estado | Bug |
|---|---|---|
| 02 | **GAP 19** | Asignar un `Value` a un local pierde el tag: `var v = r["a"]` → `str(v)` = puntero. Ver `gaps.txt` y `examples/lang-features/findings/19-value-local.clsx`. |
| 03 | resuelto | Atributo desnudo (`<input disabled />` → `true`): cubierto por `11-bare-attr`. |
| 04 | resuelto | Literal CMX dentro de `{...}` (`{f((<p>x</p>))}`): cubierto por `06-components`. |

Ver `tests/findings/README.md`.
