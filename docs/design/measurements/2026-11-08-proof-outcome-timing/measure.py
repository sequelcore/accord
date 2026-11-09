"""Times three ways of checking proof-outcome-rule: one warm-up, then RUNS
timed runs each, sequentially. Prints a JSON summary. Usage:

    python -I measure.py <accord-repo> <tesota-repo> <runs>
"""
import json
import os
import statistics
import subprocess
import sys
import time

accord_repo, tesota_repo, runs = sys.argv[1], sys.argv[2], int(sys.argv[3])
exe = ".exe" if os.name == "nt" else ""
accord = os.path.join(accord_repo, "target", "release", "accord" + exe)
lsc = os.path.join(tesota_repo, "node_modules", ".bin", "lsc" + exe)

workloads = {
    "accord check (release)": (
        [accord, "check", "conformance/cases/proof-outcome.accord"], accord_repo, 0),
    "lsc check --backend=dafny": (
        [lsc, "check", "--backend=dafny", "src/verification/proof-outcome-rule.ts"], tesota_repo, 0),
    "dafny verify (the .dfy only)": (
        ["dafny", "verify", "src/verification/proof-outcome-rule.dfy"], tesota_repo, 0),
}


def once(cmd, cwd, expected):
    start = time.perf_counter()
    done = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, shell=(os.name == "nt" and cmd[0] == "dafny"))
    elapsed = time.perf_counter() - start
    if done.returncode != expected:
        sys.exit(f"{cmd[0]} exited {done.returncode}:\n{done.stdout}\n{done.stderr}")
    return elapsed, done.stdout


summary = {}
for name, (cmd, cwd, expected) in workloads.items():
    _, output = once(cmd, cwd, expected)  # warm-up, not counted
    times = [once(cmd, cwd, expected)[0] for _ in range(runs)]
    summary[name] = {
        "runs": runs,
        "median_s": round(statistics.median(times), 3),
        "min_s": round(min(times), 3),
        "max_s": round(max(times), 3),
        "times_s": [round(t, 3) for t in times],
        "last_line": output.strip().splitlines()[-1] if output.strip() else "",
    }
print(json.dumps(summary, indent=2))
