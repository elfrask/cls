#!/usr/bin/env python3
# vector.py - Genera VECTORES deterministas para la prueba #6 (DNS) y las
# lineas `CHECK k=v` esperadas, con un parser DNS independiente (dns_indep.py).
#
#   gen <outdir>   escribe:
#     out/vectors/<n>.hex        mensaje DNS (hex)
#     out/vectors/<n>.expected   CHECK esperados que debe emitir el parser CLS
#     out/queries_expected.txt   queries canonicas (builder): "<name> <hex>"
#     out/live_expected.txt      CHECK esperados de la corrida LIVE (UDP+TCP)
#     out/vectors_manifest.txt   rutas .hex ordenadas (para run.ps1)
#
# Independiente de CLS: cualquier divergencia es un bug de CLS (o del oraculo).

import os
import socket
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from dns_indep import (  # noqa: E402
    T_A, T_AAAA, T_CNAME, T_MX, T_TXT, T_NS, T_SOA, CLASS_IN,
    u16, u32, encode_name, parse_message, build_query, build_response,
    build_tc_response, check_lines,
)
from zone import load_zone  # noqa: E402

# Secuencia de queries LIVE (misma que embebe el runner CLS).
LIVE = [
    ("live01", 0x2001, "example.test", T_A, "udp"),
    ("live02", 0x2002, "example.test", T_AAAA, "udp"),
    ("live03", 0x2003, "www.example.test", T_CNAME, "udp"),
    ("live04", 0x2004, "www.example.test", T_A, "udp"),
    ("live05", 0x2005, "example.test", T_MX, "udp"),
    ("live06", 0x2006, "example.test", T_TXT, "udp"),
    ("live07", 0x2007, "example.test", T_NS, "udp"),
    ("live08", 0x2008, "example.test", T_SOA, "udp"),
    ("live09", 0x2009, "big.example.test", T_TXT, "udp"),   # TC
    ("live10", 0x2009, "big.example.test", T_TXT, "tcp"),   # completa
]

# Queries canonicas para verificar el BUILDER de CLS byte a byte.
QUERIES = [
    ("q-a", 0x3001, "example.test", T_A),
    ("q-aaaa", 0x3002, "example.test", T_AAAA),
    ("q-cname", 0x3003, "www.example.test", T_CNAME),
    ("q-mx", 0x3004, "example.test", T_MX),
    ("q-txt", 0x3005, "example.test", T_TXT),
    ("q-ns", 0x3006, "example.test", T_NS),
    ("q-soa", 0x3007, "example.test", T_SOA),
    ("q-bigtxt", 0x3008, "big.example.test", T_TXT),
]


def header(qid, flags, qd, an, ns, ar):
    return u16(qid) + u16(flags) + u16(qd) + u16(an) + u16(ns) + u16(ar)


def question(name, qtype):
    return encode_name(name) + u16(qtype) + u16(CLASS_IN)


def rr(owner, rtype, ttl, rdata):
    return owner + u16(rtype) + u16(CLASS_IN) + u32(ttl) + u16(len(rdata)) + rdata


PTR12 = bytes([0xC0, 0x0C])


def mki(name, qid, flags, qname, qtype, answers, auth=None):
    if auth is None:
        auth = []
    out = bytearray()
    out += header(qid, flags, 1, len(answers), len(auth), 0)
    out += question(qname, qtype)
    for a in answers:
        out += a
    for a in auth:
        out += a
    return name, bytes(out)


def soa_rdata(mname, rname, serial, refresh, retry, expire, minimum):
    return (encode_name(mname) + encode_name(rname)
            + u32(serial) + u32(refresh) + u32(retry) + u32(expire) + u32(minimum))


def build_vectors():
    vecs = []

    # 01 - A simple con compresion (owner -> 0xC00C).
    vecs.append(mki("01-a-simple", 0x0101, 0x8180, "example.test", T_A,
                    [rr(PTR12, T_A, 300, socket.inet_aton("192.0.2.1"))]))

    # 02 - Multiples A.
    a2 = [rr(PTR12, T_A, 300, socket.inet_aton(ip))
          for ip in ("192.0.2.1", "192.0.2.2", "192.0.2.3")]
    vecs.append(mki("02-multi-a", 0x0102, 0x8180, "example.test", T_A, a2))

    # 03 - Mixto: A + AAAA + MX + TXT (multi-string).
    txt2 = bytes([6]) + b"v=spf1" + bytes([4]) + b"-all"
    mixed = [
        rr(PTR12, T_A, 300, socket.inet_aton("192.0.2.1")),
        rr(PTR12, T_AAAA, 300, socket.inet_pton(socket.AF_INET6, "2001:db8::1")),
        rr(PTR12, T_MX, 300, u16(10) + encode_name("mail.example.test")),
        rr(PTR12, T_TXT, 300, txt2),
    ]
    vecs.append(mki("03-mixed", 0x0103, 0x8580, "example.test", T_A, mixed))

    # 04 - Cadena CNAME con puntero al nombre en RDATA (no al offset 12).
    out = bytearray()
    out += header(0x0104, 0x8180, 1, 2, 0, 0)
    out += question("www.example.test", T_A)
    crdata_off = len(out) + 12  # owner(2)+type(2)+class(2)+ttl(4)+rdlen(2)
    out += rr(PTR12, T_CNAME, 300, encode_name("example.test"))
    ptr = bytes([0xC0 | ((crdata_off >> 8) & 0x3F), crdata_off & 0xFF])
    out += rr(ptr, T_A, 300, socket.inet_aton("192.0.2.1"))
    vecs.append(("04-cname-chain", bytes(out)))

    # 05 - NXDOMAIN: rcode 3 + SOA en authority.
    soa = soa_rdata("ns1.example.test", "hostmaster.example.test",
                    2024010101, 3600, 600, 604800, 300)
    vecs.append(mki("05-nxdomain", 0x0105, 0x8183, "missing.example.test", T_A,
                    [], [rr(encode_name("example.test"), T_SOA, 300, soa)]))

    # 06 - TC (truncada).
    vecs.append(mki("06-tc", 0x0106, 0x8780, "big.example.test", T_TXT, []))

    # 07 - Compresion en cadena (puntero -> puntero -> offset 12).
    out = bytearray()
    out += header(0x0107, 0x8180, 1, 2, 0, 0)
    out += question("x.example.test", T_A)
    ans1_rdata_off = len(out) + 12
    out += rr(PTR12, T_CNAME, 300, PTR12)  # rdata = puntero a 12
    cptr = bytes([0xC0 | ((ans1_rdata_off >> 8) & 0x3F), ans1_rdata_off & 0xFF])
    out += rr(cptr, T_A, 300, socket.inet_aton("192.0.2.7"))
    vecs.append(("07-compression-chain", bytes(out)))

    # 08 - TXT multi-string.
    txt3 = bytes([5]) + b"hello" + bytes([5]) + b"world" + bytes([1]) + b"x"
    vecs.append(mki("08-txt-multistring", 0x0108, 0x8180, "example.test", T_TXT,
                    [rr(PTR12, T_TXT, 300, txt3)]))

    # 09 - SOA en answer.
    vecs.append(mki("09-soa", 0x0109, 0x8180, "example.test", T_SOA,
                    [rr(PTR12, T_SOA, 300, soa)]))

    # 10 - AAAA no canonica (ejercita la compresion RFC 5952).
    vecs.append(mki("10-aaaa", 0x0110, 0x8180, "example.test", T_AAAA,
                    [rr(PTR12, T_AAAA, 300,
                        socket.inet_pton(socket.AF_INET6, "2001:db8:0:0:1:0:0:1"))]))

    # 11 - Dos NS.
    ns2 = [rr(PTR12, T_NS, 300, encode_name("ns1.example.test")),
           rr(PTR12, T_NS, 300, encode_name("ns2.example.test"))]
    vecs.append(mki("11-ns", 0x0111, 0x8180, "example.test", T_NS, ns2))

    # 12 - Nombre largo.
    longn = "a.b.c.d.e.f.g.h.www.example.test"
    vecs.append(mki("12-longname", 0x0112, 0x8180, longn, T_A,
                    [rr(PTR12, T_A, 300, socket.inet_aton("10.20.30.40"))]))

    # 13 - Flags raros: opcode=1, rcode=4, RD+RA, sin aa.
    vecs.append(mki("13-flags", 0x0113, 0x8984, "example.test", T_A, []))

    return vecs


def gen(outdir):
    os.makedirs(outdir, exist_ok=True)
    vdir = os.path.join(outdir, "vectors")
    os.makedirs(vdir, exist_ok=True)

    vecs = build_vectors()
    manifest = []
    for name, data in vecs:
        hexp = os.path.join(vdir, name + ".hex")
        with open(hexp, "w", encoding="utf-8", newline="\n") as f:
            f.write(data.hex() + "\n")
        m = parse_message(data)
        with open(os.path.join(vdir, name + ".expected"), "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(check_lines(name, m)) + "\n")
        manifest.append(hexp)

    with open(os.path.join(outdir, "vectors_manifest.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(manifest) + "\n")

    # Queries canonicas (builder).
    with open(os.path.join(outdir, "queries_expected.txt"), "w", encoding="utf-8", newline="\n") as f:
        for name, qid, qname, qtype in QUERIES:
            f.write("%s %s\n" % (name, build_query(qid, qname, qtype).hex()))

    # Queries que envia el runner LIVE (una por linea; la de big se repite: UDP+TCP).
    with open(os.path.join(outdir, "live_queries_expected.txt"), "w", encoding="utf-8", newline="\n") as f:
        for name, qid, qname, qtype, transport in LIVE:
            f.write(build_query(qid, qname, qtype).hex() + "\n")

    # Esperado combinado de la etapa OFFLINE: dumps de los vectores (mismo
    # orden que build_vectors) + round-trip de answer() (rt1..rtN).
    zone_off = load_zone()
    off_lines = []
    for name, data in vecs:
        off_lines += check_lines(name, parse_message(data))
    for i, (name, qid, qname, qtype) in enumerate(QUERIES):
        q = build_query(qid, qname, qtype)
        m = parse_message(build_response(q, zone_off))
        off_lines += check_lines("rt" + str(i + 1), m)
    with open(os.path.join(outdir, "offline_expected.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(off_lines) + "\n")

    # Esperado de la corrida LIVE (dumps + aserciones check::).
    zone = load_zone()
    lines = []
    for name, qid, qname, qtype, transport in LIVE:
        q = build_query(qid, qname, qtype)
        if transport == "udp" and qname == "big.example.test":
            resp = build_tc_response(q)
        else:
            resp = build_response(q, zone)
        m = parse_message(resp)
        lines += check_lines(name, m)
    # Bloque de aserciones explicitas (check::) del runner CLS.
    lines += [
        "CHECK attr.a.ip=ok",
        "CHECK attr.aaaa.ip=ok",
        "CHECK attr.cname.target=ok",
        "CHECK attr.chase.an=ok",
        "CHECK attr.mx=ok",
        "CHECK attr.txt=ok",
        "CHECK attr.ns=ok",
        "CHECK attr.soa.mname=ok",
        "CHECK attr.soa.serial=ok",
        "CHECK attr.big.tc=ok",
        "CHECK attr.big.tcp.tc=ok",
        "CHECK attr.big.tcp.an=ok",
        "CHECK attr.big.tcp.txt3=ok",
    ]
    with open(os.path.join(outdir, "live_expected.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")

    print("vector: %d vectores, %d queries, %d lineas live"
          % (len(vecs), len(QUERIES), len(lines)))


def main(argv):
    if len(argv) < 3 or argv[1] != "gen":
        print("uso: vector.py gen <outdir>", file=sys.stderr)
        return 2
    gen(argv[2])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
