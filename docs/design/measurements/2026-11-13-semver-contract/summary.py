"""Judges run.sh's output against the README's success rule.

Usage: python summary.py <z3 results> <cvc5 results>
Each file is run.sh's output; a file may hold only one solver's column, the
other being "-". Prints, for each solver, the proof obligations that are not
`unsat` within 5 s, the open-case searches that are not `unsat`, and for each
planted fault the clauses on which the solver says `sat`; then whether the
success rule holds, and whether it would hold if a fault needed only one
solver's `sat`, which replay.py then checks.
"""
import re
import sys

LIMIT_MS = 5000


def read(path, column):
    """case -> (answer, min, median, max) for the Z3 (0) or cvc5 (1) column."""
    rows = {}
    for line in open(path, encoding="utf-8").read().splitlines()[1:]:
        parts = line.split()
        columns, i = [], 1
        while i < len(parts):
            width = 1 if parts[i] == "-" else 4
            columns.append(parts[i:i + width])
            i += width
        cell = columns[column]
        if cell != ["-"]:
            rows[parts[0]] = (cell[0], *map(int, cell[1:]))
    return rows


def judge(rows):
    proofs = {c: r for c, r in rows.items() if c.endswith("-prove")}
    opens = {c: r for c, r in rows.items() if c.endswith("-open-search")}
    faults = {}
    for c, r in rows.items():
        m = re.match(r"(.+)-fault-(\w+)-refute$", c)
        if m:
            faults.setdefault(m[2], {})[m[1]] = r[0]
    bad_proofs = {c: r for c, r in proofs.items() if r[0] != "unsat" or r[3] >= LIMIT_MS}
    bad_opens = {c: r for c, r in opens.items() if r[0] != "unsat"}
    slow_opens = {c: r for c, r in opens.items() if r[0] == "unsat" and r[3] >= LIMIT_MS}
    sat = {f: sorted(c for c, a in by.items() if a == "sat") for f, by in faults.items()}
    return len(proofs), bad_proofs, len(opens), bad_opens, slow_opens, sat


def main(z3_path, cvc5_path):
    results = {}
    for name, path, column in (("z3", z3_path, 0), ("cvc5", cvc5_path, 1)):
        rows = read(path, column)
        n, bad, n_open, bad_open, slow_open, sat = judge(rows)
        results[name] = (bad, bad_open, sat)
        print(f"{name}: {n - len(bad)}/{n} proofs unsat under {LIMIT_MS} ms; "
              f"{n_open - len(bad_open)}/{n_open} open searches unsat")
        for c, r in sorted(bad.items()):
            print(f"  proof not met: {c}: {r[0]} (max {r[3]})")
        for c, r in sorted(bad_open.items()):
            print(f"  open search not met: {c}: {r[0]}")
        for c, r in sorted(slow_open.items()):
            print(f"  open search unsat but slow: {c}: max {r[3]} ms")
        for f, clauses in sorted(sat.items()):
            print(f"  fault {f} sat on: {', '.join(clauses) or 'no clause'}")
    z, c = results["z3"], results["cvc5"]
    both = {f: sorted(set(z[2][f]) & set(c[2][f])) for f in z[2]}
    either = {f: sorted(set(z[2][f]) | set(c[2][f])) for f in z[2]}
    print("faults sat in both solvers on a clause:", {f: bool(v) for f, v in both.items()})
    print("faults sat in some solver:", {f: bool(v) for f, v in either.items()})
    proofs_met = not z[0] and not c[0] and not z[1] and not c[1]
    print("success rule met:", proofs_met and all(both.values()))
    print("if refutation by one solver's replayed counterexample were allowed, "
          "met (replay.py checks the replay):", proofs_met and all(either.values()))


if __name__ == "__main__":
    main(*sys.argv[1:3])
