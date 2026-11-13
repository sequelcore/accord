"""Runs each proof obligation and open search of a folder of cases under
perturbations that don't change what a case says, and classifies each case by
how the answers vary (the method of Mariposa, Zhou et al. 2023, written here:
`mutate.py`).

Variants of a case, for each solver: the case as written; Z3 with
`smt.random_seed=N sat.random_seed=N` (cvc5 with `--seed=N
--sat-random-seed=N`) for N in 1..10; the assertions shuffled, with 5 seeds;
the symbols renamed, with 5 seeds. That is 21 runs per case and solver, each
with the 10 s limit of run.sh.

Classes: stable (every run `unsat`), unsolvable (no run `unsat`), unstable
(some, not all). A stable case is slow if its slowest run reaches 5 s.

Usage: ACCORD_Z3=... ACCORD_CVC5=... python stability.py <folder> <label> [--z3 "opts"] [--jobs 4]
Writes results-round3/C-<label>.csv (one row per run) and C-<label>.txt.
"""
import argparse
import csv
import os
import re
import shutil
import tempfile
from concurrent.futures import ThreadPoolExecutor

import mutate
from measure import solve

SEEDS = range(1, 11)
SHUFFLES = range(1, 6)
RENAMES = range(1, 6)
OBLIGATION = re.compile(r"-(prove|open-search)\.smt2$")


def variants(solver):
    yield "base", None, [], []
    for n in SEEDS:
        opts = [f"smt.random_seed={n}", f"sat.random_seed={n}"] if solver == "z3" else [f"--seed={n}", f"--sat-random-seed={n}"]
        yield f"seed{n}", None, opts, []
    for n in SHUFFLES:
        yield f"shuffle{n}", ("shuffle", n), [], []
    for n in RENAMES:
        yield f"rename{n}", ("rename", n), [], []


def classify(rows):
    answers = [a for a, _ in rows]
    unsat = answers.count("unsat")
    if unsat == len(answers):
        return "stable"
    return "unsolvable" if unsat == 0 else "unstable"


def summarize(csv_path, out_path, label):
    runs = {}
    with open(csv_path, newline="") as f:
        for case, solver, variant, answer, ms in csv.reader(f):
            runs.setdefault((case, solver), []).append((answer, int(ms), variant))
    cases = sorted({c for c, _ in runs})
    lines = [f"# {label}", ""]
    totals = {}
    for solver in ("z3", "cvc5"):
        lines.append(f"{solver}: case, class, unsat of runs, other answers, slowest unsat ms, median ms")
        for case in cases:
            rows = runs.get((case, solver))
            if not rows:
                continue
            cls = classify([(a, m) for a, m, _ in rows])
            unsat = [m for a, m, _ in rows if a == "unsat"]
            others = {}
            for a, _, _ in rows:
                if a != "unsat":
                    others[a] = others.get(a, 0) + 1
            slow = " slow" if cls == "stable" and max(unsat) >= 5000 else ""
            med = sorted(m for _, m, _ in rows)[len(rows) // 2]
            lines.append(f"  {case:46} {cls + slow:16} {len(unsat):2}/{len(rows)} {str(others) if others else '':24} {max(unsat) if unsat else '-':>6} {med:>6}")
            totals.setdefault(solver, {}).setdefault("slow" if slow else cls, []).append(case)
        lines.append("")
    lines.append("totals (cases per class)")
    for solver, by in totals.items():
        n = sum(len(v) for v in by.values())
        lines.append(f"  {solver}: {n} cases: " + ", ".join(f"{k} {len(v)}" for k, v in sorted(by.items())))
        for k in ("unstable", "unsolvable", "slow"):
            if by.get(k):
                lines.append(f"    {k}: " + ", ".join(by[k]))
    open(out_path, "w", newline="\n").write("\n".join(lines) + "\n")
    print("\n".join(lines[-12:]))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("folder")
    p.add_argument("label")
    p.add_argument("--z3", default="")
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--out", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "results-round3"))
    a = p.parse_args()
    names = sorted(f for f in os.listdir(a.folder) if OBLIGATION.search(f))
    scratch = tempfile.mkdtemp(prefix="stab-")
    jobs = []
    try:
        with ThreadPoolExecutor(a.jobs) as pool:
            for name in names:
                text = open(os.path.join(a.folder, name), encoding="utf-8").read()
                made = {}
                for solver in ("z3", "cvc5"):
                    for variant, mutation, opts, _ in variants(solver):
                        path = os.path.join(a.folder, name)
                        if mutation:
                            if variant not in made:
                                made[variant] = os.path.join(scratch, f"{variant}-{name}")
                                body = getattr(mutate, mutation[0])(text, mutation[1])
                                open(made[variant], "w", newline="\n", encoding="utf-8").write(body)
                            path = made[variant]
                        extra = opts + (a.z3.split() if solver == "z3" else [])
                        jobs.append((name[:-5], solver, variant, pool.submit(solve, solver, path, extra)))
            csv_path = os.path.join(a.out, f"C-{a.label}.csv")
            with open(csv_path, "w", newline="") as f:
                w = csv.writer(f, lineterminator="\n")
                for case, solver, variant, fut in jobs:
                    answer, ms = fut.result()
                    if "error" in answer:
                        raise SystemExit(f"{solver} {case} {variant}: {answer}")
                    w.writerow([case, solver, variant, answer, ms])
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    summarize(csv_path, os.path.join(a.out, f"C-{a.label}.txt"), a.label)


if __name__ == "__main__":
    if os.environ.get("SUMMARIZE"):
        import sys
        summarize(sys.argv[1], sys.argv[2], sys.argv[3])
    else:
        main()
