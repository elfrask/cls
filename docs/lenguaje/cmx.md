# CMX (JSX nativo)

CMX es una sintaxis tipo JSX para construir **valores** de marcado desde CLS.
Se escribe dentro de paréntesis como expresión:

```clx
function main(args: String[]) -> int {
    var contador = 1;
    var app = (
        <app titulo="Hola mundo" contador={contador + 1}>
            <parrafo>Contenido</parrafo>
            <separador />
            <item id={3} />
        </app>
    );
    print("tag:", app.tag);
    print("props.titulo:", app.props.titulo);
    print("props.contador:", app.props.contador);
    print("children:", app.children.length);
    print("child0 tag:", app.children[0].tag);
    print("child2 props.id:", app.children[2].props.id);
    return 0;
};
```

## Atributos

| Forma | Valor |
|---|---|
| `nombre="texto"` | String |
| `nombre={expr}` | la expresión evaluada |
| `{nombre}` | shorthand: lee la variable con ese nombre (Null si no existe) |
| `nombre` (sin valor) | `true` |

## Children

- Texto plano -> String.
- `{expr}` -> el valor de la expresión.
- Elementos anidados -> valores CMX recursivos.
- `self-closing`: `<separador />`.

## Valor resultante

Un valor CMX (builtin `Cmx`, 48 B) expone:

| Campo | Tipo | Descripción |
|---|---|---|
| `tag` | `String` | nombre del tag (siempre string, también para tags mayúscula) |
| `props` | `Record<String, Value>` | atributos (`app.props.titulo`) |
| `children` | `Cmx[]` | hijos (`app.children[0].tag`) |
| `kind` | `Int` | `0` = elemento, `1` = nodo de texto |
| `tagCallable` | `Value` | referencia (función/var/clase) para tags mayúscula |
| `isComponent` | `Bool` | `true` si el tag es una referencia invocable |

Un nodo de texto tiene `kind == 1` y su contenido está en `tag`.

## Clase core `CMX`

El builtin se expone como la clase core **`CMX`** (métodos en CLS, inyectados en
todo módulo; ver `cls-core/src/frontend/builtins.rs`). El typeck asocia `CMX`
con el tipo `Type::Cmx`:

| Método | Resultado |
|---|---|
| `isPrimitive()` | `!isComponent` |
| `isEmpty()` | `tag.length == 0` |
| `isElementOf(tag)` | `me.tag == tag` |
| `flattenChildren()` | `Cmx[]` |
| `tryCallTag()` | invoca `tagCallable(props, children)` si es componente; si no, `null` |
| `toString()` / `__str` / `__repr` | representación de depuración (ver abajo) |
| `cloneWith(props?, children?)` | copia inmutable (conserva tag/kind/tagCallable/isComponent) |

También funcionan `c is Cmx` y la anotación genérica borrada `CMX<P, PP>`.

## Representación en `print`

`print(cmx)` / `cmx.toString()` producen la representación de depuración:

- Sin children: `<tag prop="valor" />` (con `props` ordenadas; `/>` si está
  vacío).
- Con children: `<tag>... (n children)</tag>`.
- Ej.: `print(app)` -> `<app contador="2" titulo="Hola mundo">... (3 children)</app>`
  (props ordenadas alfabéticamente).

## Runtime

El **JIT** compila CMX **inline** (0 host calls): `__alloc(48)` + stores; `props`
con `__intr_record_*`, `children` con un array de pares `(val, tag)` (stride 16).
El lexer usa un buffer FIFO (`cmx_buffer`) con detección de `<`/`>` balanceados,
y el parser construye el elemento (`parse_cmx_element`) con expresiones en
atributos, shorthand `{var}`, atributos desnudos (`disabled` → `true`), spreads
`{...props}`, literales CMX anidados dentro de `{...}` y arrow functions con `()`.
`toString` usa el internal fusionado `__intr_cmx_fmt`.

Ejemplo completo: `examples/test/jit-test/units/12-cmx.clsx`.

## Renderizar CMX a HTML

`Cmx` es un **DOM virtual** (tag/props/children) con `toString()` de depuración,
pero **no se serializa a HTML por sí solo**. Para HTML (p. ej. en un framework
HTTP), se recorre el árbol en **CLS puro**:

```clx
function e(s: String) -> String {
    # escape HTML: & -> &amp;, < -> &lt;, > -> &gt;, " -> &quot;, ' -> &#39;
}

function render(c: Cmx) -> String {
    # texto (kind==1): e(c.tag)
    # componente (isComponent): c.tagCallable(c.props, c.children)
    # elemento: "<tag prop=\"valor\" ...>" + render de cada child + "</tag>"
}
```

**Cómo cubrir las limitaciones de CMX en un renderer**:

| Necesidad | Patrón |
|---|---|
| Loops | `renderEach(items, fn)` en CLS puro (los children `Array` no se auto-expanden) |
| Condicionales | `{cond ? a : b}` como expresión, o `if` fuera del marcado |
| Layouts/herencia | Composición de funciones (`layout_base(titulo, body)`) |
| Componentes reutilizables | invocar `c.tagCallable(...)` (o `c.tryCallTag()`) |
| Raw HTML (sin escape) | Una función `raw(html)` que el renderer reconozca y no escape |

**Límites**: no hay `for`/`if` estructurales dentro del marcado (solo
expresiones); un child `Array` no se expande solo (usar `renderEach`). Todo
contenido interpolado debe escaparse salvo `raw()` explícito (riesgo de inyección).

### Invocar un componente (tag mayúscula)

El tag mayúscula guarda la **referencia** en `tagCallable` y marca
`isComponent = true`; `tag` es el nombre (String). El renderer (o el usuario) lo
invoca:

```clx
function App(props: Record<String, Value>, children: Cmx[]) -> String {
    var t: String = props.titulo;
    return "App " + t;
};

var base: Record<String, Value> = {titulo: "demo"};
var app = (<App {...base}><p>hijo</p></App>);   # spread de props en el tag

if (app.isComponent) {
    var html: String = app.tagCallable(app.props, app.children);
    print(html);   # "App demo"
}
# equivalente: print(app.tryCallTag());   (null si no es componente)
```

- `app.isComponent` — `true` si el tag es una referencia invocable; `false` para
  tags minúscula. `app.tag` es **siempre** el nombre (String).
- `app.tagCallable` — el handle a invocar con `(props, children)`.
- `tryCallTag()` — hace exactamente eso y devuelve `null` si no es componente.
- `<App {...base} />` — spread de props en el tag: fusiona el record en las
  props; los atributos nombrados tienen prioridad. El source debe estar anotado
  `Record<String, Value>` (un Shape sin anotar no garantiza el layout hashmap).
- `{...app.props, ...}` — spread de props al invocar (REST_SPREAD_PLAN: spread
  en records y arrays).

Ejemplo completo: `examples/cmx-templates/` (prueba #7) y
`docs/desarrollo/minilaravel.md` (Tombstone, renderer
`cls-repositories/tombstone/modules/tombstone/view.clsx`).