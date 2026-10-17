# `dns` — codec DNS (RFC 1035) en CLS puro

Ladrillo de la stdlib autocontenida (`examples/patrones.md` §11). **CLS puro, sin
FFI**: es manipulación de bits, big-endian y parseo binario sobre `int[]`
(bytes). El transporte UDP/TCP vive en [`modules/net`](../net/README.md).

## Qué cubre

- **Big-endian**: `getU16`/`getU32`, `pushU16`/`pushU32`.
- **Nombres**: `encodeName`, `decodeName` con **compresión** (punteros `0xC0`) y
  **guard anti-loop** (máx. saltos/labels). `decodeName` devuelve `{name, next}`
  donde `next` avanza **solo los bytes reales** del nombre (no el puntero), para
  seguir parseando RDATA tras una compresión.
- **Header + flags**: `qr/opcode/aa/tc/rd/ra/rcode` y contadores.
- **RRs**: A(1) IPv4, AAAA(28) IPv6 canónico (RFC 5952), CNAME(5)/NS(2),
  MX(15) `{pref,nombre}`, TXT(16) lista de strings longitud-prefijadas,
  SOA(6) `{mname,rname,serial,refresh,retry,expire,minimum}`. Soporta RDATA de
  longitud variable y offsets tras compresión.
- **`buildQuery(id, name, qtype, rd)`**: constructor de consultas.
- **`answer(query, zone)`**: servidor **autoritativo puro** (QR=1, AA=1, copia la
  question y comprime los nombres al offset 12 de la question).

## Modelo de datos

Clases con herencia: `Rr` (base) y subclases `RrA`/`RrAAAA`/`RrName`/`RrMX`/
`RrTXT`/`RrSOA`, con `render()` polimórfico. `DnsMessage`, `Zone`, `ZoneRecord`,
`NameResult`.

## API principal

| Función / tipo | Descripción |
|---|---|
| `parseMessage(bytes: int[]) -> DnsMessage` | parsea header + question + RRs (answer/authority/additional). |
| `buildQuery(id, name, qtype, rd) -> int[]` | arma una query. |
| `answer(query, zone) -> int[]` | respuesta autoritativa byte-construida. |
| `encodeName` / `decodeName` | nombres con compresión. |
| `typeName` / `typeCode` | código ↔ nombre de tipo. |
| `rrLine(rr) -> String` | `"TYPE name ttl render"`. |
| `toHex(bytes) -> String` | volcado hex (para vectores). |

Consumidor: `examples/dns/`.
