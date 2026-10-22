# tests — prueba #6 (DNS)

La prueba no usa un directorio `tests/` con archivos `.clsx` sueltos: su
verificación es **diferencial** y se genera/orquesta desde `run.ps1`:

| Artefacto | Qué es | Quién lo genera |
|---|---|---|
| `out/vectors/<n>.hex` | mensaje DNS de un caso límite | Python `oracle/vector.py gen` |
| `out/vectors/<n>.expected` | `CHECK k=v` que debe emitir el parser CLS | Python `oracle/vector.py gen` |
| `out/offline_expected.txt` | esperado combinado (vectores + `answer()` round-trip) | Python |
| `out/queries_expected.txt` | queries canónicas (verifica el **builder** CLS) | Python |
| `out/live_expected.txt` | `CHECK` de la corrida **live** (UDP+TCP) | Python |
| `out/live_queries_expected.txt` | queries que envía el runner live | Python |
| `out/cls_offline.txt` | salida del runner CLS offline | CLS `src/main.clsx` |
| `out/cls_live.txt` | salida del runner CLS live | CLS `src/main.clsx` |
| `out/queries.log` | queries realmente recibidas por el server | `oracle/server.py` |

Para correr todo:

```powershell
powershell -File examples\dns\run.ps1
```

Casos cubiertos (vectores `out/vectors/`): A simple con compresión, múltiples A,
mixto (A+AAAA+MX+TXT), cadena CNAME con puntero a nombre en RDATA, NXDOMAIN con
SOA, TC, compresión en cadena (puntero→puntero→offset 12), TXT multi-string,
SOA, AAAA no canónica (RFC 5952), dos NS, nombre largo y flags raros
(opcode/rcode/RD/RA).

Además: `probe_udp.clsx` es un probe de de-riesgo del FFI UDP (contra un server
echo) que se puede correr aparte.
