# findings de `examples/cmx-templates`

Repros de bugs/limitaciones hallados al construir el motor CMX->HTML en CLS puro.

`run.ps1` los corre y espera **exit != 0 mientras el bug exista**:

- `[BUG]`  sigue fallando  -> bug PRESENTE (no es regresion de esta prueba).
- `[FIX?]` ya pasa (exit 0) -> bug arreglado: **promover a la matriz** (goldens).
- `[GAP]`  listado en `gaps.txt` -> limite de diseno documentado (no cuenta).

| Archivo | Bug |
|---|---|
| `02-value-local-tag-loss.clsx` | Asignar un `Value` a una variable local pierde el tag de tipo: `var v = r["a"]` (Record<String,Value>) re-interpreta el string como puntero; `str(v)` da basura. Inline (`str(r["a"])`) funciona. |
| `03-cmx-bare-attribute.clsx` | Un atributo CMX sin valor (`<input disabled />`) no llega a `props` (los docs dicen que deberia ser `true`). |
| `04-cmx-nested-in-interpolation.clsx` | Un literal CMX como argumento dentro de un child de interpolacion `{f((<p/>))}` no parsea (`Esperaba ')' , encontro <cmx>`). |

> **Resuelto por el rework CMX**: `01-tag-string-is-callable` (el `tag` de CMX ya
> es `String`; el componente vive en `tagCallable` + `isComponent`) se retiro y
> quedo cubierto por el golden `10-component.html`.
