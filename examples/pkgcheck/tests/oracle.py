#!/usr/bin/env python3
"""Oraculo SHA-256 para `examples/pkgcheck` (prueba #8).

Reproduce EXACTAMENTE los inputs de `src/main.clsx` (1 + 1 + 2 NIST + 1e6'a' +
200 LCG) y compara el digest (hashlib) byte a byte contra la salida de la
implementacion en CLS puro. Tambien valida la prueba de corrupcion (1 bit).

Uso:  python tests/oracle.py out/digests.txt
Codigos: 0 = OK, 1 = DIVERGE, 3 = sin hashlib (imposible; python trae hashlib).
"""

import hashlib
import sys

MASK31 = 2147483648


def lcg(s: int) -> int:
    return (s * 1103515245 + 12345) % MASK31


def build_inputs():
    out = []
    out.append(b"")
    out.append(b"abc")
    out.append(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")
    out.append(
        b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno"
        b"ijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
    )
    out.append(b"a" * 1000000)
    s = 123456789
    for _ in range(200):
        s = lcg(s)
        n = s % 256
        buf = bytearray()
        for _ in range(n):
            s = lcg(s)
            buf.append((s >> 16) & 255)
        out.append(bytes(buf))
    return out


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else None
    if path:
        with open(path, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
    else:
        lines = sys.stdin.read().splitlines()

    digests = [l.strip() for l in lines if l.strip() and not l.startswith("corrupt=")]
    corrupt = [l for l in lines if l.startswith("corrupt=")]

    expected = [hashlib.sha256(m).hexdigest() for m in build_inputs()]

    fails = 0
    if len(digests) != len(expected):
        print(f"FAIL: {len(digests)} digests, esperaba {len(expected)}")
        return 1
    for i, (got, exp) in enumerate(zip(digests, expected)):
        if got != exp:
            fails += 1
            if fails <= 5:
                print(f"  FAIL[{i}]: cls={got} hashlib={exp}")
    if fails == 0:
        print(f"  {len(expected)} digests OK (NIST + 1e6'a' + 200 LCG)")

    if not corrupt:
        print("FAIL: falta la linea corrupt=")
        return 1
    a, b = corrupt[0].split("=", 1)[1].split(":")
    m = bytes(range(32))
    ea = hashlib.sha256(m).hexdigest()
    m2 = bytearray(m)
    m2[0] ^= 1
    eb = hashlib.sha256(bytes(m2)).hexdigest()
    if a != ea:
        print(f"FAIL corrupt(a): {a} != {ea}")
        fails += 1
    if b != eb:
        print(f"FAIL corrupt(b): {b} != {eb}")
        fails += 1
    if a == b:
        print("FAIL: el hash no cambio al corromper 1 bit")
        fails += 1
    if fails == 0:
        print("  corrupcion OK (1 bit cambia el hash)")

    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
