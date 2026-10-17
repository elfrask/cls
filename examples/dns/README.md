# Prueba #6 — DNS resolver / authoritative (`examples/dns/`)

Prueba **diferencial** de CLS contra un **oráculo externo independiente**
(Python puro con `socket`/`struct`, **sin `dnslib`**). Sin red externa: el
servidor autoritativo es local (`127.0.0.1:15353`).

Eje que estresa: **manipulación de bits / endianness / parseo binario / sockets
UDP (FFI) / compresión de nombres / retry TCP**. Es **CLS puro + FFI a C**
(prohibido `host_calls`, `fs`/`http`/`json`/`strings` del host, punteros).

```
examples/dns/
├── cls.json                 { entry: src/main.clsx, pathModules: ["../../modules"] }
├── src/
│   ├── main.clsx            runner: modos `offline` y `live`
│   └── probe_udp.clsx       probe mínimo de de-riesgo del FFI UDP
├── oracle/
│   ├── dns_indep.py         codec DNS independiente (Python puro)
│   ├── zone.json            zona fija (definida UNA vez; la consume Python)
│   ├── zone.py              carga la zona (+ big TXT determinista)
│   ├── server.py            servidor autoritativo UDP+TCP (oráculo en vivo)
│   └── vector.py            genera vectores + esperado (`gen`)
├── run.ps1                  orquesta todo (offline + live + nslookup opcional)
├── out/                     artefactos generados (vectores, logs, salidas)
├── findings/                repros mínimos de bugs + README
└── tests/README.md
```

## Módulos compartidos (ladrillos de la stdlib)

- **`modules/net`** — transporte UDP/TCP por **FFI** a un shim C (sin punteros;
  handles enteros, payloads como `int[]`). `udpOpen/Send/Recv/Close`,
  `tcpOpen/Send/Recv/Close` (framing DNS-over-TCP `[len:u16 BE][msg]`),
  `resolve`. Requiere `powershell -File modules\net\build.ps1`.
- **`modules/dns`** — codec DNS **CLS puro** (RFC 1035): big-endian, nombres con
  compresión `0xC0` + guard anti-loop, RRs A/AAAA/CNAME/NS/MX/TXT/SOA,
  `buildQuery` y `answer(query, zone)` (servidor autoritativo).

Ver `modules/net/README.md` y `modules/dns/README.md`.

## Cómo correr

Desde la **raíz del repo** (los `.dll` del FFI se resuelven por CWD):

```powershell
powershell -File examples\dns\run.ps1
```

Hace, en orden:

1. Compila los shims FFI (`modules/net`, `modules/io`).
2. Python `vector.py gen` → vectores + esperado.
3. **OFFLINE** — el runner CLS parsea los vectores (dump `CHECK k=v`), construye
   las queries canónicas (`out/cls_queries.hex`) y prueba `answer()` por
   round-trip. Comparación **CLS ↔ Python**:
   - `cls_offline.txt` vs `offline_expected.txt` (360 líneas).
   - builder `cls_queries.hex` vs `queries_expected.txt` **byte a byte**.
4. **LIVE** — levanta `server.py` (UDP+TCP), corre el runner (`live01..live10`:
   A/AAAA/CNAME/A(cadena)/MX/TXT/NS/SOA; `big.example.test` TC por UDP → retry
   TCP), corta el server. Compara `cls_live.txt` vs `live_expected.txt` y las
   queries recibidas (`out/queries.log`) vs las esperadas.
5. **nslookup** opcional contra el server local (SKIP si el entorno no lo
   permite; en Windows `nslookup` no acepta puerto no estándar).

Salida: `RESULTADO: OK` / `DIVERGENCIA` (exit 0/1).

## Zona fija

Apex `example.test` (definida en `oracle/zone.json`, consumida por Python):
`A=192.0.2.1`, `AAAA=2001:db8::1`, `MX 10 mail.example.test`, `TXT "v=spf1 -all"`,
`NS ns1.example.test`, `SOA ns1.example.test hostmaster... 2024010101 ...`,
`CNAME www.example.test -> example.test`, y `big.example.test` TXT (3×200 bytes →
respuesta > 512 B, truncable).

## Findings

Ver `findings/README.md`: (F1) arrays de parámetro por valor, (F2) corrupción de
strings en auto-concatenación, (F3) `is` con clase cross-module.
