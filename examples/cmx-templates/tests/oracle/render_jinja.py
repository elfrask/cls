#!/usr/bin/env python3
"""Oraculo cruzado Jinja2 para `examples/cmx-templates`.

Compara la salida del renderer CMX->HTML en CLS puro (golden `09-parity.html`)
contra Jinja2 con autoescape, en el subconjunto donde ambos coinciden:

  - interpolacion de texto
  - un bucle `{% for %}`
  - escape de `&`, `<`, `>`, `'`

NO se comparan comillas dobles: CLS emite `&quot;`, Jinja2/markupsafe emite
`&#34;` (ambos validos en HTML).

Codigos de salida: 0 = OK, 1 = DIVERGE, 3 = SKIP (jinja2 no instalado).
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GOLDEN = os.path.join(HERE, "..", "golden", "09-parity.html")

TEMPLATE = '<ul class="parity">{% for it in items %}<li>{{ it }}</li>{% endfor %}</ul>'
ITEMS = ["<uno>", "dos & tres", "cua'ro"]


def main() -> int:
    try:
        from jinja2 import Environment
    except ImportError:
        print("SKIP: jinja2 no esta instalado (pip install jinja2)")
        return 3

    if not os.path.exists(GOLDEN):
        print(f"SKIP: no existe {GOLDEN} (corre run.ps1 primero)")
        return 3

    with open(GOLDEN, encoding="utf-8") as fh:
        expected = fh.read().strip()

    env = Environment(autoescape=True)
    got = env.from_string(TEMPLATE).render(items=ITEMS).strip()

    if got != expected:
        print("DIVERGE:")
        print(f"  jinja2 : {got!r}")
        print(f"  cls    : {expected!r}")
        return 1

    print("OK: Jinja2 (autoescape) coincide con el renderer CMX en el subconjunto de paridad")
    return 0


if __name__ == "__main__":
    sys.exit(main())
