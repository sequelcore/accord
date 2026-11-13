"""Adds a call-site reveal to cases written by ground.py: for each spec call
the function's own `requires` asserts, `(canon_from x k)`, one ground step of
its unfolding axiom, with no quantifier.

Usage: python reveal.py <ground folder> <output folder>
"""
import pathlib
import re
import sys

src, dst = map(pathlib.Path, sys.argv[1:3])
dst.mkdir(parents=True, exist_ok=True)
n = 0
for f in sorted(src.glob("*.smt2")):
    out = []
    for line in f.read_text().splitlines():
        out.append(line)
        m = re.fullmatch(r"\(assert \(canon_from (\(\w+ \w+\)|\S+) (\S+)\)\)", line)
        if m:
            x, k = m.groups()
            out.append(f"(assert (= (canon_f (S F0) {x} {k}) (or (>= {k} (seq.len {x})) "
                       f"(and (canon_id (seq.nth {x} {k})) (canon_f F0 {x} (+ {k} 1))))))")
            n += 1
    (dst / f.name).write_text("\n".join(out) + "\n")
print(n, "reveals added")
