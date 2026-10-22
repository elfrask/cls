# findings de `examples/cmx-templates`

Repros de bugs/limitaciones hallados al construir el motor CMX->HTML en CLS puro.

`run.ps1` los corre y espera **exit != 0 mientras el bug exista**:

- `[BUG]`  sigue fallando  -> bug PRESENTE (no es regresion de esta prueba).
- `[FIX?]` ya pasa (exit 0) -> bug arreglado: **promover a la matriz** (goldens).
- `[GAP]`  listado en `gaps.txt` -> limite de diseno documentado (no cuenta).

| Archivo | Bug |
|---|---|
| `02-value-local-tag-loss.clsx` | **[GAP 19]** Asignar un `Value` a una variable local pierde el tag de tipo: `var v = r["a"]` (Record<String,Value>) re-interpreta el string como puntero; `str(v)` da basura. Inline (`str(r["a"])`) funciona. Limite de diseno del emisor (no taggea locales `Value`); ver `gaps.txt` y `examples/lang-features/findings/19-value-local.clsx`. |

> **Resueltos por el rework CMX / correcciones posteriores** (repros retirados,
> ahora cubiertos por la matriz de goldens):
>
> - `01-tag-string-is-callable`: el `tag` de CMX ya es `String`; el componente
>   vive en `tagCallable` + `isComponent` → cubierto por `10-component.html`.
> - `03-cmx-bare-attribute` (`<input disabled />` → `props["disabled"] = true`):
>   arreglado en el lexer (`AttrBare`) + emisor (`I32Const` true) → cubierto por
>   `11-bare-attr.html`.
> - `04-cmx-nested-in-interpolation` (`{f((<p>x</p>))}`): arreglado drenando
>   siempre el buffer CMX del lexer → cubierto por `06-components.html`.
