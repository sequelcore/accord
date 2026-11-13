"""Runs the solvers on a folder of cases, up to --jobs processes at once.

Usage: ACCORD_Z3=... ACCORD_CVC5=... python measure.py <folder> <out> [options]
  --runs N      runs per case and solver (default 5)
  --jobs N      solver processes at once (default 4)
  --z3 "opts"   more options for Z3, after `-smt2 -t:10000`
  --cvc5 "opts" more options for cvc5, after `--lang smt2 --mbqi --tlimit-per=10000`
  --no-mbqi     run cvc5 without --mbqi
  --only z3|cvc5
  --match REGEX only the cases whose name matches
The output has run.sh's layout (answers, then the minimum, median and maximum
time in milliseconds), so summary.py reads it. `stability.py` imports `solve`.
"""
import argparse
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

Z3_BASE = ["-smt2", "-t:10000"]
CVC5_BASE = ["--lang", "smt2", "--mbqi", "--tlimit-per=10000"]


def solve(solver, path, extra=(), mbqi=True):
    """(first line of the answer, wall time in ms) of one run."""
    exe = os.environ["ACCORD_Z3" if solver == "z3" else "ACCORD_CVC5"]
    base = Z3_BASE if solver == "z3" else [o for o in CVC5_BASE if mbqi or o != "--mbqi"]
    args = base + list(extra)
    start = time.perf_counter()
    try:
        done = subprocess.run([exe, *args, path], capture_output=True, text=True, timeout=60)
        if done.stdout.strip():
            out = done.stdout.replace("\r", "").strip().split("\n")[0]
        elif done.returncode:
            out = f"crash{done.returncode & 0xFFFFFFFF:x}"  # crashc00000fd is a stack overflow
        else:
            out = done.stderr.strip().split("\n")[0]
    except subprocess.TimeoutExpired:
        out = "hang"
    ms = round((time.perf_counter() - start) * 1000)
    if out.startswith("cvc5 interrupted"):
        out = "timeout"
    return out, ms


def cell(results, runs):
    answers = [a for a, _ in results]
    times = sorted(t for _, t in results)
    distinct = {}
    for a in answers:
        distinct[a] = distinct.get(a, 0) + 1
    label = "/".join(a + (f"x{n}" if n < runs else "") for a, n in sorted(distinct.items()))
    return f"{label} {times[0]} {times[runs // 2]} {times[-1]}"


def main():
    p = argparse.ArgumentParser()
    p.add_argument("folder")
    p.add_argument("out")
    p.add_argument("--runs", type=int, default=5)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--z3", default="")
    p.add_argument("--cvc5", default="")
    p.add_argument("--no-mbqi", action="store_true")
    p.add_argument("--only", default="")
    p.add_argument("--match", default="")
    a = p.parse_args()
    names = sorted(f for f in os.listdir(a.folder) if f.endswith(".smt2") and re.search(a.match, f))
    tasks = []
    with ThreadPoolExecutor(a.jobs) as pool:
        for n in names:
            path = os.path.join(a.folder, n)
            for solver, opts in (("z3", a.z3.split()), ("cvc5", a.cvc5.split())):
                if a.only in ("", solver):
                    tasks.append((n, solver, [pool.submit(solve, solver, path, opts, not a.no_mbqi) for _ in range(a.runs)]))
        rows = {}
        for n, solver, futures in tasks:
            rows.setdefault(n, {})[solver] = cell([f.result() for f in futures], a.runs)
    with open(a.out, "w", newline="\n") as out:
        out.write(f"{'case':46} {'z3 (answer min med max ms)':26} {'cvc5 --mbqi (answer min med max ms)':26}\n")
        for n in names:
            out.write(f"{n[:-5]:46} {rows[n].get('z3', '-'):26} {rows[n].get('cvc5', '-'):26}\n")


if __name__ == "__main__":
    main()
