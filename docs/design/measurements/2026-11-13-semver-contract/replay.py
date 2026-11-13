"""Replays the solvers' counterexamples to the planted faults.

For each fault case in the given folders and each solver configuration, runs
the solver, and when it says `sat` reads the model's inputs, runs the faulty
function on them with a small reference evaluator, and checks the clause the
case asserted against the concrete result. A counterexample replays when the
inputs meet `requires` and the clause is false for the concrete result. The
evaluator is written here from the model in README.md, not taken from Accord's
interpreter.

Usage: ACCORD_Z3=... ACCORD_CVC5=... python replay.py <case folder>...
       python replay.py --search      (does any small input violate the clause?)
"""
import itertools
import os
import re
import subprocess
import sys

# ---- S-expressions of a solver's `get-value` reply ----


def tokens(text):
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c.isspace():
            i += 1
        elif c in "()":
            yield c
            i += 1
        elif c == '"':
            j, out = i + 1, []
            while True:
                if text[j] == '"':
                    if j + 1 < n and text[j + 1] == '"':
                        out.append('"')
                        j += 2
                        continue
                    break
                out.append(text[j])
                j += 1
            yield ("str", unescape("".join(out)))
            i = j + 1
        else:
            j = i
            while j < n and not text[j].isspace() and text[j] not in '()"':
                j += 1
            yield text[i:j]
            i = j


def unescape(s):
    s = re.sub(r"\\u\{([0-9a-fA-F]{1,5})\}", lambda m: chr(int(m.group(1), 16)), s)
    return re.sub(r"\\u([0-9a-fA-F]{4})", lambda m: chr(int(m.group(1), 16)), s)


def parse(text):
    stack, cur = [], []
    for t in tokens(text):
        if t == "(":
            stack.append(cur)
            cur = []
        elif t == ")":
            done, cur = cur, stack.pop()
            cur.append(done)
        else:
            cur.append(t)
    return cur


def value(e):
    """A model value: an int, a PreId ('N' or 'A', text) or a list of them."""
    if isinstance(e, str):
        return int(e)
    if e[0] == "-":
        return -int(e[1])
    if e[0] in ("Numeric", "Alpha"):
        return ("N" if e[0] == "Numeric" else "A", e[1][1])
    if e[0] == "as":
        return []
    if e[0] == "seq.unit":
        return [value(e[1])]
    if e[0] == "seq.++":
        return [x for part in e[1:] for x in value(part)]
    raise ValueError(f"unknown value {e}")


def model(reply, names):
    lines = reply.strip().split("\n", 1)
    if lines[0].strip() != "sat":
        return lines[0].strip(), None
    pairs = parse(lines[1])[0]
    return "sat", {name: value(v) for name, v in pairs}


# ---- the reference evaluator ----


def canon_id(x):
    kind, s = x
    return kind == "A" or (s != "" and all("0" <= c <= "9" for c in s) and (s == "0" or s[0] != "0"))


def id_lt(x, y):
    (kx, sx), (ky, sy) = x, y
    if kx == "N" and ky == "A":
        return True
    if kx == "N" and ky == "N":
        return len(sx) < len(sy) or (len(sx) == len(sy) and sx < sy)
    return kx == "A" and ky == "A" and sx < sy


def cmp_id(x, y, fault=None):
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


def cmp_list(a, b, k, fault=None):
    if k == len(a):
        if k == len(b):
            return 0
        return 1 if fault == "prefix" else -1
    if k == len(b):
        return -1 if fault == "prefix" else 1
    c = cmp_id(a[k], b[k])
    if fault == "skip" or c != 0:
        return c
    return cmp_list(a, b, k + 1, fault)


def first(a, b, k):
    i = k
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    return i


def lt_at(a, b, i):
    return (i == len(a) and i < len(b)) or (i < len(a) and i < len(b) and id_lt(a[i], b[i]))


def gt_at(a, b, i):
    return (i == len(b) and i < len(a)) or (
        i < len(a) and i < len(b) and not id_lt(a[i], b[i]) and a[i] != b[i])


def eq_from(a, b, k):
    return len(a) == len(b) and all(a[j] == b[j] for j in range(k, len(a)))


def lprefix(x, y, k):
    return len(x) < len(y) and all(x[j] == y[j] for j in range(k, len(x)))


def diff_at(a, b, k, test):
    i = first(a, b, k)
    return not (i < len(a) and i < len(b) and test(a[i], b[i]))


# A clause is true for the inputs and the result.
ID_CLAUSES = {
    "sign": lambda x, y, r: -1 <= r <= 1,
    "lt": lambda x, y, r: r != -1 or id_lt(x, y),
    "eq": lambda x, y, r: r != 0 or x == y,
    "gt": lambda x, y, r: r != 1 or (not id_lt(x, y) and x != y),
}
LIST_CLAUSES = {
    "sign": lambda a, b, k, r: -1 <= r <= 1,
    "eq": lambda a, b, k, r: r != 0 or eq_from(a, b, k),
    "lt-u": lambda a, b, k, r: r != -1 or lt_at(a, b, first(a, b, k)),
    "lt-e": lambda a, b, k, r: r != -1 or lt_at(a, b, first(a, b, k)),
    "gt-u": lambda a, b, k, r: r != 1 or gt_at(a, b, first(a, b, k)),
    "gt-e": lambda a, b, k, r: r != 1 or gt_at(a, b, first(a, b, k)),
    "pin": lambda a, b, k, r: True,
    "same": lambda a, b, k, r: not eq_from(a, b, k) or r == 0,
    "short": lambda a, b, k, r: not lprefix(a, b, k) or r == -1,
    "long": lambda a, b, k, r: not lprefix(b, a, k) or r == 1,
    "diff-lt": lambda a, b, k, r: diff_at(a, b, k, id_lt) or r == -1,
    "diff-gt": lambda a, b, k, r: diff_at(a, b, k, lambda x, y: not id_lt(x, y)) or r == 1,
}


def replays(function, fault, clause, m):
    """(whether the model is a counterexample that replays, a description)."""
    if function == "cmp_id":
        x, y = m["x"], m["y"]
        if not (canon_id(x) and canon_id(y)):
            return False, "violates requires"
        r = cmp_id(x, y, fault)
        return not ID_CLAUSES[clause](x, y, r), f"x={x} y={y} result={r}"
    a, b, k = m["a"], m["b"], m["k"]
    if not (0 <= k <= len(a) and k <= len(b) and all(map(canon_id, a + b))):
        return False, "violates requires"
    r = cmp_list(a, b, k, fault)
    return not LIST_CLAUSES[clause](a, b, k, r), f"a={a} b={b} k={k} result={r}"


# ---- the solvers ----

CONFIGS = {
    "z3": ("ACCORD_Z3", ["-smt2", "-t:10000"]),
    "z3+arith2": ("ACCORD_Z3", ["-smt2", "-t:10000", "smt.arith.solver=2"]),
    # No quantifier instantiation: a `sat` is only a candidate, which the
    # replay judges; an `unsat` still holds.
    "z3+noq": ("ACCORD_Z3", ["-smt2", "-t:10000", "smt.mbqi=false", "smt.ematching=false"]),
    "cvc5": ("ACCORD_CVC5", ["--lang", "smt2", "--mbqi", "--tlimit-per=10000"]),
}
CASE = re.compile(r"^(?P<function>cmp_id|cmp_list)-(?P<clause>.+)-fault-(?P<fault>\w+)-refute\.smt2$")
NAMES = {"cmp_id": "(x y)", "cmp_list": "(a b k)"}


def run(config, text, path):
    variable, args = CONFIGS[config]
    scratch = os.environ.get("TEMP", ".")
    tmp = os.path.join(scratch, f"replay-{os.getpid()}.smt2")
    with open(tmp, "w", newline="\n") as out:
        out.write(text + f"(get-value {NAMES[path]})\n")
    try:
        done = subprocess.run([os.environ[variable], *args, tmp], capture_output=True, text=True, timeout=30)
        return done.stdout.replace("\r", "")
    except subprocess.TimeoutExpired:
        return "timeout"
    finally:
        os.remove(tmp)


def replay_folder(folder):
    print(f"## {folder}")
    print(f"{'case':44} {'solver':10} answer   replay")
    summary = {}
    for name in sorted(os.listdir(folder)):
        match = CASE.match(name)
        if not match:
            continue
        function, clause, fault = match["function"], match["clause"], match["fault"]
        text = open(os.path.join(folder, name), encoding="utf-8").read()
        summary.setdefault((function, fault), {})
        for config in CONFIGS:
            answer, m = model(run(config, text, function), None)
            if m is None:
                print(f"{name[:-5]:44} {config:10} {answer:8} -", flush=True)
                continue
            ok, why = replays(function, fault, clause, m)
            print(f"{name[:-5]:44} {config:10} {answer:8} {'replays' if ok else 'does not replay'}: {why}", flush=True)
            summary.setdefault((function, fault), {}).setdefault(config, []).append(clause if ok else None)
            summary[(function, fault)].setdefault("sat:" + config, []).append(clause)
    print(f"\n{'fault':16} {'solver':10} clauses sat   clauses replayed")
    for (function, fault), by in sorted(summary.items()):
        for config in CONFIGS:
            sat = by.get("sat:" + config, [])
            good = [c for c in by.get(config, []) if c]
            print(f"{function + ' ' + fault:16} {config:10} {len(sat):2} {' '.join(sat):40} {len(good):2} {' '.join(good)}")
    print()


# ---- small-input search ----

ALPHABET = [("N", "0"), ("N", "1"), ("N", "10"), ("A", ""), ("A", "a"), ("A", "b")]


def search():
    """Whether some small input violates each faulty function's clause."""
    ids = list(itertools.product(ALPHABET, repeat=2))
    lists = [list(t) for n in range(3) for t in itertools.product(ALPHABET, repeat=n)]
    for fault in ("numeric",):
        for clause, f in ID_CLAUSES.items():
            bad = next(((x, y) for x, y in ids if not f(x, y, cmp_id(x, y, fault))), None)
            print(f"cmp_id {fault:8} {clause:8} {bad}")
    for fault in ("prefix", "skip"):
        for clause, f in LIST_CLAUSES.items():
            bad = next(((a, b, k) for a in lists for b in lists for k in range(min(len(a), len(b)) + 1)
                        if not f(a, b, k, cmp_list(a, b, k, fault))), None)
            print(f"cmp_list {fault:8} {clause:8} {bad}")


if __name__ == "__main__":
    if sys.argv[1:] == ["--search"]:
        search()
    else:
        for folder in sys.argv[1:]:
            replay_folder(folder)
