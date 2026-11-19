"""Tallies run.sh's output: each case's outcome per variant, as Accord would
combine the two solvers (both unsat: proved; both sat: refuted; otherwise
undecided), whether any seed changed an answer, and any crash.

Usage: python3 summarize.py results.txt
"""
import collections
import re
import sys

LINE = re.compile(r"(\S+) (\S+) (\d) \| (.*) (\d+) \| (.*) (\d+)$")
answers = collections.defaultdict(set)
slowest = collections.defaultdict(int)
crashes = []
for line in open(sys.argv[1], encoding="utf-8"):
    m = LINE.match(line.strip())
    if not m:
        continue
    case, variant, _, z3, z3_ms, cvc5, cvc5_ms = m.groups()
    answers[(case, variant)].add((z3.split()[0].strip("()"), cvc5.split()[0]))
    slowest[variant] = max(slowest[variant], int(z3_ms), int(cvc5_ms))
    if "crash" in (z3, cvc5):
        crashes.append(line.strip())


def outcome(pairs):
    if len(pairs) > 1:
        return "unstable"
    (pair,) = pairs
    return {("unsat", "unsat"): "proved", ("sat", "sat"): "refuted"}.get(pair, "undecided")


variants = ["none", "A", "B", "saturate"]
cases = sorted({case for case, _ in answers})
rows = collections.Counter(
    tuple(outcome(answers[(case, v)]) for v in variants) for case in cases
)
print(f"{len(cases)} cases; columns: {' / '.join(variants)}")
for row, count in sorted(rows.items(), key=lambda r: -r[1]):
    print(f"{count:3}  {' / '.join(row)}")
print("unstable:", [k for k, v in answers.items() if len(v) > 1] or "none")
print("crashes:", crashes or "none")
print("slowest run per variant, ms:", dict(slowest))
