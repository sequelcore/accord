"""Runs each comparator of the port, correct and with each planted fault,
on every small input, and checks the `e-versions` contract's clauses on the
results (CONTRACT=e EQ=versions in gen.py). The evaluator is written from the
model in README.md, as in replay.py; it isn't Accord's interpreter.

For each function and fault it reports the inputs enumerated, how many break
some clause, the smallest one that does, the clauses it breaks, and the time.
For the correct comparator it also counts the inputs on which the contract
admits a result other than the comparator's, or more than one (the contract
doesn't determine the result).

Usage: python enumerate_inputs.py
Identifiers come from a small alphabet of canonical numeric (no leading zero)
and alphanumeric values. Lists have at most 3 elements over 8 identifiers and
at most 4 over 4; a version's main numbers are one of three triples, its
pre-release has at most 2 identifiers over 4, and its build at most 2 over 3.
"""
import itertools
import sys
import time

from replay import eq_from, first, gt_at, id_lt, lt_at

ALPHABET8 = [("N", "0"), ("N", "1"), ("N", "2"), ("N", "10"), ("A", "a"), ("A", "b"), ("A", "1a"), ("A", "A")]
ALPHABET4 = [("N", "0"), ("N", "10"), ("A", "a"), ("A", "1a")]
ALPHABET3 = [("N", "1"), ("A", "a"), ("A", "A")]
MAINS = [(0, 0, 0), (0, 0, 1), (1, 0, 0)]
FAULTS = [None, "numeric", "prefix", "skip"]


def cmp_id(x, y, fault):
    (kx, sx), (ky, sy) = x, y
    if kx == "N" and ky == "N":
        if len(sx) != len(sy):
            return -1 if len(sx) < len(sy) else 1
        return -1 if sx < sy else (0 if sx == sy else 1)
    if kx == "N":
        return 1 if fault == "numeric" else -1
    if ky == "N":
        return -1 if fault == "numeric" else 1
    return -1 if sx < sy else (0 if sx == sy else 1)


def cmp_list(a, b, k, fault):
    if k == len(a):
        return 0 if k == len(b) else (1 if fault == "prefix" else -1)
    if k == len(b):
        return -1 if fault == "prefix" else 1
    c = cmp_id(a[k], b[k], fault)
    return c if fault == "skip" or c != 0 else cmp_list(a, b, k + 1, fault)


def compare_main(a, b):
    return (a[:3] > b[:3]) - (a[:3] < b[:3])


def compare(a, b, fault):
    c = compare_main(a, b)
    if c:
        return c
    if not a[3]:
        return 0 if not b[3] else 1
    if not b[3]:
        return -1
    return cmp_list(a[3], b[3], 0, fault)


def compare_build(a, b, fault):
    return compare(a, b, fault) or cmp_list(a[4], b[4], 0, fault)


# ---- the contract's clauses: (inputs, result) -> bool ----

def lt_e(x, y, k):
    return lt_at(x, y, first(x, y, k))


def gt_e(x, y, k):
    return gt_at(x, y, first(x, y, k))


def pre_lt(x, y):
    return (len(x) > 0 and not y) or (len(x) > 0 and len(y) > 0 and lt_e(x, y, 0))


def pre_gt(x, y):
    return (not x and len(y) > 0) or (len(x) > 0 and len(y) > 0 and gt_e(x, y, 0))


def main_eq(a, b):
    return a[:3] == b[:3]


def ver_lt(a, b):
    return a[:3] < b[:3] or (main_eq(a, b) and pre_lt(a[3], b[3]))


def ver_gt(a, b):
    return a[:3] > b[:3] or (main_eq(a, b) and pre_gt(a[3], b[3]))


def prec_eq(a, b):
    return main_eq(a, b) and eq_from(a[3], b[3], 0)


CLAUSES = {
    "cmp_id": {
        "sign": lambda i, r: -1 <= r <= 1,
        "lt": lambda i, r: r != -1 or id_lt(*i),
        "eq": lambda i, r: r != 0 or i[0] == i[1],
        "gt": lambda i, r: r != 1 or (not id_lt(*i) and i[0] != i[1]),
    },
    "cmp_list": {
        "sign": lambda i, r: -1 <= r <= 1,
        "eq": lambda i, r: r != 0 or eq_from(*i),
        "lt-e": lambda i, r: r != -1 or lt_e(*i),
        "gt-e": lambda i, r: r != 1 or gt_e(*i),
    },
    "compare_main": {
        "sign": lambda i, r: -1 <= r <= 1,
        "eq": lambda i, r: r != 0 or main_eq(*i),
        "lt": lambda i, r: r != -1 or i[0][:3] < i[1][:3],
        "gt": lambda i, r: r != 1 or i[0][:3] > i[1][:3],
    },
    "compare": {
        "sign": lambda i, r: -1 <= r <= 1,
        "eq": lambda i, r: r != 0 or prec_eq(*i),
        "lt-e": lambda i, r: r != -1 or ver_lt(*i),
        "gt-e": lambda i, r: r != 1 or ver_gt(*i),
    },
    "compare_build": {
        "sign": lambda i, r: -1 <= r <= 1,
        "eq": lambda i, r: r != 0 or (prec_eq(*i) and eq_from(i[0][4], i[1][4], 0)),
        "lt-e": lambda i, r: r != -1 or ver_lt(*i) or (prec_eq(*i) and lt_e(i[0][4], i[1][4], 0)),
        "gt-e": lambda i, r: r != 1 or ver_gt(*i) or (prec_eq(*i) and gt_e(i[0][4], i[1][4], 0)),
    },
}
RUN = {
    "cmp_id": lambda i, f: cmp_id(i[0], i[1], f),
    "cmp_list": lambda i, f: cmp_list(i[0], i[1], i[2], f),
    "compare_main": lambda i, f: compare_main(*i),
    "compare": lambda i, f: compare(i[0], i[1], f),
    "compare_build": lambda i, f: compare_build(i[0], i[1], f),
}


def lists(alphabet, longest):
    return [list(t) for n in range(longest + 1) for t in itertools.product(alphabet, repeat=n)]


def inputs_id():
    return list(itertools.product(ALPHABET8, repeat=2))


def inputs_list(alphabet, longest):
    ls = lists(alphabet, longest)
    return [(a, b, k) for a in ls for b in ls for k in range(min(len(a), len(b)) + 1)]


def inputs_main():
    return list(itertools.product(MAINS, repeat=2))


def inputs_versions():
    vs = [(*m, p, b) for m in MAINS for p in lists(ALPHABET4, 2) for b in lists(ALPHABET3, 2)]
    return list(itertools.product(vs, repeat=2))


def size(i):
    """Elements in the input: list elements, or both versions' identifiers."""
    total = 0
    for x in i:
        if isinstance(x, list):
            total += len(x)
        elif isinstance(x, tuple) and len(x) == 5:
            total += len(x[3]) + len(x[4]) + sum(x[:3])
        elif isinstance(x, tuple):
            total += 1
    return total


def check(function, label, inputs):
    clauses = CLAUSES[function]
    print(f"\n## {function}: {label}, {len(inputs)} inputs")
    print(f"{'fault':8} {'time s':>7} {'violating':>10}  smallest counterexample -> result: clauses broken")
    for fault in FAULTS:
        start = time.perf_counter()
        bad, best, best_clauses, open_inputs = 0, None, None, 0
        for i in inputs:
            r = RUN[function](i, fault)
            broken = [n for n, c in clauses.items() if not c(i, r)]
            if broken:
                bad += 1
                key = (size(i), repr(i))
                if best is None or key < best[0]:
                    best, best_clauses = (key, i), broken
            if fault is None:
                admitted = [x for x in (-1, 0, 1) if all(c(i, x) for c in clauses.values())]
                if admitted != [r]:
                    open_inputs += 1
        took = time.perf_counter() - start
        detail = f"{best[1]} -> {RUN[function](best[1], fault)}: {', '.join(best_clauses)}" if best else "none"
        print(f"{fault or 'correct':8} {took:7.1f} {bad:10}  {detail}")
        if fault is None:
            print(f"         inputs where the contract admits a result other than the comparator's, or more than one: {open_inputs}")
        sys.stdout.flush()


if __name__ == "__main__":
    check("cmp_id", "8 identifiers, every pair", inputs_id())
    check("cmp_list", "lists of at most 3 over 8 identifiers, every k", inputs_list(ALPHABET8, 3))
    check("cmp_list", "lists of at most 4 over 4 identifiers, every k", inputs_list(ALPHABET4, 4))
    check("compare_main", "3 main triples", inputs_main())
    check("compare", "versions: 3 mains, pre-release at most 2 over 4, build at most 2 over 3", inputs_versions())
    check("compare_build", "the same versions", inputs_versions())
