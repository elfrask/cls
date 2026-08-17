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