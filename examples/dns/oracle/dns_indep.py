#!/usr/bin/env python3
# dns_indep.py - Implementacion DNS INDEPENDIENTE en Python puro (socket/struct),
# SIN dnslib, usada como ORACULO externo de la prueba #6.
#
# Reimplementa desde cero (no comparte codigo con CLS):
#   - encode_name / decode_name con compresion 0xC0 y guard anti-loop
#   - parse_message (header + question + RRs A/AAAA/CNAME/NS/MX/TXT/SOA)
#   - build_query
#   - build_response: servidor autoritativo (mismo algoritmo canonico que CLS)
#   - check_lines: lineas `CHECK k=v` (protocolo de comparacion con el runner CLS)
#
# Es deliberadamente independiente: se apoya en la stdlib (socket.inet_ntop,
# struct) para que un bug de CLS NO se reproduzca aqui.

import socket
import struct

T_A = 1
T_NS = 2
T_CNAME = 5
T_SOA = 6
T_MX = 15
T_TXT = 16
T_AAAA = 28
T_ANY = 255
CLASS_IN = 1

TYPE_BY_NAME = {
    "A": T_A, "NS": T_NS, "CNAME": T_CNAME, "SOA": T_SOA,
    "MX": T_MX, "TXT": T_TXT, "AAAA": T_AAAA,
}
NAME_BY_TYPE = {v: k for k, v in TYPE_BY_NAME.items()}


def type_name(code):
    return NAME_BY_TYPE.get(code, "TYPE" + str(code))


def get_u16(d, o):
    return (d[o] << 8) | d[o + 1]


def get_u32(d, o):
    return (d[o] << 24) | (d[o + 1] << 16) | (d[o + 2] << 8) | d[o + 3]


def u16(v):
    return struct.pack("!H", v & 0xFFFF)


def u32(v):
    return struct.pack("!I", v & 0xFFFFFFFF)


def encode_name(name):
    out = bytearray()
    if name in ("", "."):
        return bytes([0])
    for lab in name.split("."):
        if lab == "":
            continue
        b = lab.encode("ascii")
        out.append(len(b))
        out += b
    out.append(0)
    return bytes(out)


def decode_name(data, off):
    labels = []
    next_off = -1
    jumps = 0
    guard = 0
    n = len(data)
    while True:
        if off < 0 or off >= n:
            break
        guard += 1
        if guard > 512:
            break
        l = data[off]
        if l == 0:
            off += 1
            if next_off < 0:
                next_off = off
            break
        if (l & 0xC0) == 0xC0:
            if off + 1 >= n:
                break
            ptr = ((l & 0x3F) << 8) | data[off + 1]
            if next_off < 0:
                next_off = off + 2
            off = ptr
            jumps += 1
            if jumps > 64:
                break
            continue
        s = off + 1
        if s + l > n:
            break
        labels.append(data[s:s + l].decode("ascii", "replace"))
        off = s + l
    name = ".".join(labels)
    if name == "":
        name = "."
    return name, next_off


def render_rdata(rtype, rd, data, rdata_off):
    if rtype == T_A:
        return ".".join(str(b) for b in rd[:4])
    if rtype == T_AAAA:
        return socket.inet_ntop(socket.AF_INET6, bytes(rd[:16]))
    if rtype in (T_CNAME, T_NS):
        return decode_name(data, rdata_off)[0]
    if rtype == T_MX:
        pref = get_u16(rd, 0)
        return str(pref) + " " + decode_name(data, rdata_off + 2)[0]
    if rtype == T_TXT:
        out = []
        i = 0
        while i < len(rd):
            l = rd[i]
            i += 1
            out.append(rd[i:i + l].decode("ascii", "replace"))
            i += l
        if not out:
            out.append("")
        return "|".join(out)
    if rtype == T_SOA:
        mn, mnext = decode_name(data, rdata_off)
        rn, rnext = decode_name(data, mnext)
        so = rnext
        return "%s %s %d %d %d %d %d" % (
            mn, rn, get_u32(data, so), get_u32(data, so + 4),
            get_u32(data, so + 8), get_u32(data, so + 12), get_u32(data, so + 16))
    return rd.hex()


def parse_rr(data, start):
    name, off = decode_name(data, start)
    rtype = get_u16(data, off)
    ttl = get_u32(data, off + 4)
    rdlen = get_u16(data, off + 8)
    rdata_off = off + 10
    rd = data[rdata_off:rdata_off + rdlen]
    return {
        "name": name, "type": rtype, "ttl": ttl,
        "val": render_rdata(rtype, rd, data, rdata_off),
        "next": rdata_off + rdlen,
    }


def parse_message(data):
    m = {
        "id": get_u16(data, 0), "flags": get_u16(data, 2),
        "answers": [], "authority": [], "additional": [],
        "qname": "", "qtype": 0,
    }
    f = m["flags"]
    m["qr"] = (f >> 15) & 1
    m["opcode"] = (f >> 11) & 15
    m["aa"] = (f >> 10) & 1
    m["tc"] = (f >> 9) & 1
    m["rd"] = (f >> 8) & 1
    m["ra"] = (f >> 7) & 1
    m["rcode"] = f & 15
    m["qd"] = get_u16(data, 4)
    m["an"] = get_u16(data, 6)
    m["ns"] = get_u16(data, 8)
    m["ar"] = get_u16(data, 10)
    off = 12
    if m["qd"] > 0 and off < len(data):
        m["qname"], off = decode_name(data, off)
        m["qtype"] = get_u16(data, off)
        off += 4
    for _ in range(m["an"]):
        r = parse_rr(data, off)
        m["answers"].append(r)
        off = r["next"]
    for _ in range(m["ns"]):
        r = parse_rr(data, off)
        m["authority"].append(r)
        off = r["next"]
    for _ in range(m["ar"]):
        r = parse_rr(data, off)
        m["additional"].append(r)
        off = r["next"]
    return m


def check_lines(name, m):
    L = []
    L.append("CHECK %s.id=%d" % (name, m["id"]))
    L.append("CHECK %s.qr=%d" % (name, m["qr"]))
    L.append("CHECK %s.aa=%d" % (name, m["aa"]))
    L.append("CHECK %s.tc=%d" % (name, m["tc"]))
    L.append("CHECK %s.rd=%d" % (name, m["rd"]))
    L.append("CHECK %s.ra=%d" % (name, m["ra"]))
    L.append("CHECK %s.rcode=%d" % (name, m["rcode"]))
    L.append("CHECK %s.qd=%d" % (name, m["qd"]))
    L.append("CHECK %s.an=%d" % (name, m["an"]))
    L.append("CHECK %s.ns=%d" % (name, m["ns"]))
    L.append("CHECK %s.qname=%s" % (name, m["qname"]))
    L.append("CHECK %s.qtype=%s" % (name, type_name(m["qtype"])))
    for i, r in enumerate(m["answers"]):
        L.append("CHECK %s.an%d.name=%s" % (name, i, r["name"]))
        L.append("CHECK %s.an%d.type=%s" % (name, i, type_name(r["type"])))
        L.append("CHECK %s.an%d.ttl=%d" % (name, i, r["ttl"]))
        L.append("CHECK %s.an%d.val=%s" % (name, i, r["val"]))
    for i, r in enumerate(m["authority"]):
        L.append("CHECK %s.ns%d.name=%s" % (name, i, r["name"]))
        L.append("CHECK %s.ns%d.type=%s" % (name, i, type_name(r["type"])))
        L.append("CHECK %s.ns%d.ttl=%d" % (name, i, r["ttl"]))
        L.append("CHECK %s.ns%d.val=%s" % (name, i, r["val"]))
    return L


def build_query(qid, name, qtype, rd=True):
    out = bytearray()
    out += u16(qid)
    out += u16(0x0100 if rd else 0)
    out += u16(1) + u16(0) + u16(0) + u16(0)
    out += encode_name(name)
    out += u16(qtype) + u16(CLASS_IN)
    return bytes(out)


# ---------------------------------------------------------------------------
# Zona + servidor autoritativo (mismo algoritmo canonico que modules/dns).
# ---------------------------------------------------------------------------

class Zone:
    def __init__(self, apex):
        self.apex = apex
        self.records = []  # dicts: name, type(int), ttl, value(str)

    def add(self, name, type_str, ttl, value):
        self.records.append({
            "name": name, "type": TYPE_BY_NAME[type_str],
            "ttl": ttl, "value": value,
        })

    def has_name(self, name):
        return any(r["name"] == name for r in self.records)

    def indices(self, name, rtype=-1):
        return [i for i, r in enumerate(self.records)
                if r["name"] == name and (rtype < 0 or r["type"] == rtype)]

    def first(self, name, rtype):
        for i, r in enumerate(self.records):
            if r["name"] == name and r["type"] == rtype:
                return i
        return -1


def _rdata(rec):
    """RDATA del record. Devuelve (bytes, name_offset, name_str).
    name_offset/name_str describen un nombre comprimible dentro del RDATA
    (-1/"" si no hay)."""
    t = rec["type"]
    v = rec["value"]
    if t == T_A:
        return socket.inet_aton(v), -1, ""
    if t == T_AAAA:
        return socket.inet_pton(socket.AF_INET6, v), -1, ""
    if t in (T_CNAME, T_NS):
        return encode_name(v), 0, v
    if t == T_MX:
        pref, target = v.split(" ", 1)
        return u16(int(pref)) + encode_name(target), 2, target
    if t == T_TXT:
        out = bytearray()
        for s in v.split("|"):
            b = s.encode("ascii")
            out.append(len(b))
            out += b
        return bytes(out), -1, ""
    if t == T_SOA:
        parts = v.split(" ")
        mn, rn = parts[0], parts[1]
        nums = [int(x) for x in parts[2:7]]
        return (encode_name(mn) + encode_name(rn) + b"".join(u32(x) for x in nums)), 0, mn
    return b"", -1, ""


def _emit_rr(out, rec, qname):
    if rec["name"] == qname:
        out += bytes([0xC0, 0x0C])
    else:
        out += encode_name(rec["name"])
    out += u16(rec["type"]) + u16(CLASS_IN) + u32(rec["ttl"])
    rdata, nat, nstr = _rdata(rec)
    if nstr and nstr == qname:
        enc = encode_name(nstr)
        rdata = rdata[:nat] + bytes([0xC0, 0x0C]) + rdata[nat + len(enc):]
    out += u16(len(rdata)) + rdata


def build_response(query, zone):
    q = parse_message(query)
    qname = q["qname"]
    qtype = q["qtype"]
    existing = zone.has_name(qname)
    picks = []
    if existing:
        ci = zone.first(qname, T_CNAME)
        if ci >= 0 and qtype != T_CNAME:
            picks.append(ci)
            target = zone.records[ci]["value"]
            picks += zone.indices(target, qtype)
        elif qtype == T_ANY:
            picks += zone.indices(qname, -1)
        else:
            picks += zone.indices(qname, qtype)
    rcode = 0 if existing else 3
    ns = []
    if not existing:
        si = zone.first(zone.apex, T_SOA)
        if si >= 0:
            ns.append(si)

    out = bytearray()
    out += u16(q["id"])
    flags = 0x8000 | 0x0400 | rcode
    if q["rd"] == 1:
        flags |= 0x0100
    out += u16(flags)
    out += u16(1) + u16(len(picks)) + u16(len(ns)) + u16(0)
    out += encode_name(qname) + u16(qtype) + u16(CLASS_IN)
    for idx in picks:
        _emit_rr(out, zone.records[idx], qname)
    for idx in ns:
        _emit_rr(out, zone.records[idx], qname)
    return bytes(out)


def build_tc_response(query):
    """Respuesta truncada: header + question, TC=1, sin answers."""
    q = parse_message(query)
    out = bytearray()
    out += u16(q["id"])
    flags = 0x8000 | 0x0400 | 0x0200  # QR AA TC
    if q["rd"] == 1:
        flags |= 0x0100
    out += u16(flags)
    out += u16(1) + u16(0) + u16(0) + u16(0)
    out += encode_name(q["qname"]) + u16(q["qtype"]) + u16(CLASS_IN)
    return bytes(out)
