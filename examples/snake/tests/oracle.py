#!/usr/bin/env python3
"""Oraculo Snake para `examples/snake` (prueba #9).

Reimplementa EXACTAMENTE la logica de `src/main.clsx` (misma grilla, mismo LCG
como parte del estado) y emite el estado final canonico. El runner compara la
salida del interprete CLS contra este oraculo y contra el `.final` versionado.

Uso:  python tests/oracle.py <replay>
"""

import sys


def parse_replay(text):
    toks = text.split()
    w, h, seed = int(toks[0]), int(toks[1]), int(toks[2])
    moves = toks[3] if len(toks) > 3 else ""
    return w, h, seed, moves


class Game:
    def __init__(self, w, h, seed):
        self.w = w
        self.h = h
        self.seed = seed
        self.dir = "R"
        self.score = 0
        self.alive = True
        cx, cy = w // 2, h // 2
        self.body = [(cx, cy), (cx - 1, cy), (cx - 2, cy)]
        self.fx = self.fy = 0
        self.spawn()

    def rng(self):
        self.seed = (self.seed * 1103515245 + 12345) % 2147483648
        return self.seed

    def spawn(self):
        while True:
            x = self.rng() % self.w
            y = self.rng() % self.h
            if (x, y) not in self.body:
                self.fx, self.fy = x, y
                return

    def step(self, ch):
        if not self.alive:
            return
        nd = self.dir
        if ch == "U":
            nd = "U"
        elif ch == "D":
            nd = "D"
        elif ch == "L":
            nd = "L"
        elif ch == "R":
            nd = "R"
        if {self.dir, nd} == {"U", "D"} or {self.dir, nd} == {"L", "R"}:
            nd = self.dir
        self.dir = nd
        dx, dy = {"U": (0, -1), "D": (0, 1), "L": (-1, 0), "R": (1, 0)}[nd]
        hx, hy = self.body[0][0] + dx, self.body[0][1] + dy
        if hx < 0 or hy < 0 or hx >= self.w or hy >= self.h:
            self.alive = False
            return
        segs = len(self.body)
        eating = hx == self.fx and hy == self.fy
        check = segs if eating else segs - 1
        if (hx, hy) in self.body[:check]:
            self.alive = False
            return
        keep = segs if eating else segs - 1
        self.body = [(hx, hy)] + self.body[:keep]
        if eating:
            self.score += 1
            self.spawn()

    def serialize(self):
        head = self.body[0]
        bs = ";".join(f"{x},{y}" for x, y in self.body)
        live = "true" if self.alive else "false"
        return (
            f"score={self.score} alive={live} dir={self.dir} len={len(self.body)} "
            f"head={head[0]},{head[1]} food={self.fx},{self.fy} body={bs}"
        )


def main() -> int:
    if len(sys.argv) < 2:
        print("uso: oracle.py <replay>")
        return 2
    with open(sys.argv[1], encoding="utf-8") as fh:
        w, h, seed, moves = parse_replay(fh.read())
    g = Game(w, h, seed)
    for ch in moves:
        g.step(ch)
    print(g.serialize())
    return 0


if __name__ == "__main__":
    sys.exit(main())
