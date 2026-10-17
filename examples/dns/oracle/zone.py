#!/usr/bin/env python3
# zone.py - Carga la zona fija (oracle/zone.json) para el oraculo Python.
#
# La zona se define UNA sola vez en JSON; el valor gigante de
# `big.example.test` (que dispara TC por UDP) se genera de forma determinista
# aqui para no incrustar 600 caracteres en el JSON.

import json
import os

from dns_indep import Zone

# big.example.test TXT: 3 strings de 200 bytes -> mensaje > 512 (truncable).
BIG_TXT = ("A" * 200) + "|" + ("B" * 200) + "|" + ("C" * 200)


def load_zone(path=None):
    if path is None:
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "zone.json")
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    z = Zone(data["apex"])
    for r in data["records"]:
        val = r["value"]
        if val == "BIG_TXT":
            val = BIG_TXT
        z.add(r["name"], r["type"], r["ttl"], val)
    return z
