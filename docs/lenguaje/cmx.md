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

`CmxValue { tag, props, children }`:

- `tag` - String para tags en **minúscula**; para tags en **mayúscula** es la
  **referencia** (función/var/clase) sin ejecutarla (CMX no la llama).
- `props` - `Record` con los atributos (`app.props.titulo`).
- `children` - `Array` de valores (`app.children[0].tag`, `app.children[2].props.id`).

## Representación en `print`

- Sin children: `<tag prop="valor" />` (con `props` ordenadas; `/>` si está
  vacío).
- Con children: `<tag>... (n children)</tag>`.
- Ej.: `print(app)` -> `<app contador="2" titulo="Hola mundo">... (3 children)</app>`
  (props ordenadas alfabéticamente).

## Runtime

El tree-walker evalúa el elemento en `evaluate_cmx` (atributos y children
recursivamente) y construye el `CmxValue`. El JIT compila CMX a host
functions `cmx_*`. El lexer usa un buffer FIFO (`cmx_buffer`) con detección de
`<`/`>` balanceados por tokens, y el parser construye el elemento
(`parse_cmx_element`) con soporte de expresiones en atributos y arrow
functions con `()`.

Ejemplo completo: `examples/audit/features/12-cmx.clsx`.

## Renderizar CMX a HTML

`CmxValue` es un **DOM virtual** (tag/props/children), pero **no se serializa a
HTML por sí solo**: `print(app)` produce la representación de depuración
(`<app>... (3 children)</app>`, children abreviados). No existe un `render()`
nativo ni `toString()` de CMX accesible desde el lenguaje.

Para obtener HTML (p. ej. en un framework HTTP), se recorre el árbol en **CLS
puro**:

```clx
function e(s: String) -> String {
    # escape HTML: & -> &amp;, < -> &lt;, > -> &gt;, " -> &quot;, ' -> &#39;
}

function render(cmx: any) -> String {
    # texto (kind=1): devuelve e(texto)
    # elemento: "<tag prop=\"valor\" ...>" + render de cada child + "</tag>"
    # child Array: expandir cada elemento (permite {arr.map(...)})
    # child funcion: no renderizar (o resolver segun protocolo)
}
```

**Cómo cubrir las limitaciones de CMX en un renderer**:

| Necesidad | Patrón |
|---|---|
| Loops | `{arr.map((x: T) -> <li>{x}</li>)}` — el renderer expande arrays de children |
| Condicionales | `{cond ? a : b}` como expresión, o `if` fuera del marcado |
| Layouts/herencia | Composición de funciones (`layout_base(titulo, body)`) |
| Componentes reutilizables | Llamar la función y pasar el resultado: `{home_view(...)}` (los tags en mayúscula NO se invocan) |
| Raw HTML (sin escape) | Una función `raw(html)` que el renderer reconozca y no escape |

**Límites**: no hay `for`/`if` estructurales dentro del marcado (solo
expresiones), y las funciones como tags no se ejecutan. Todo contenido
interpolado debe escaparse salvo `raw()` explícito (riesgo de inyección).

### Invocar un componente (tag mayúscula)

El tag mayúscula guarda la **referencia** (handle de función) sin ejecutarla.
Para invocarlo como componente, se usa el tipo `Callable` con `is` + narrowing
y spread de props:

```clx
function App(props: Record<String, any>) -> String {
    var t: String = props.titulo;
    return "App " + t;
};

var app = (<App titulo="demo"><p>hijo</p></App>);

if (app.tag is Callable) {
    var html: String = app.tag({...app.props, children: app.children});
    print(html);   # "App demo"
}
```

- `app.tag is Callable` — true si el tag es un handle de función (runtime,
  tag-bit); false para tags minúscula (string).
- Dentro del `if`, `app.tag` es invocable (el typeck estrecha por `is`).
- `{...app.props, children: app.children}` — spread del record de props +
  children (Fase 2 de REST_SPREAD_PLAN).

Ejemplo de uso real: `docs/desarrollo/minilaravel.md` (framework HTTP, F6).