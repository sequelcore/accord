"""Times `accord check --json` on every conformance case and program, with no
ledger, and counts each obligation's outcome. Prints a JSON summary.

    python -I measure.py <accord binary> <conformance folder> <runs>

The conformance folder is copied to a temporary folder first, so the ledger
each check writes beside its target never reaches the checkout. A case whose
first line is `// expect: error`, or a program the compiler rejects, is
skipped: it has no obligation. Each target runs once as a warm-up, then
<runs> times, each with its ledger deleted, and its median time is kept.
"""
import collections
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

accord, conformance, runs = os.path.abspath(sys.argv[1]), sys.argv[2], int(sys.argv[3])


def targets(root):
    cases = os.path.join(root, "cases")
    for name in sorted(os.listdir(cases)):
        path = os.path.join(cases, name)
        if name.endswith(".accord"):
            with open(path, encoding="utf-8") as f:
                if f.readline().strip() != "// expect: error":
                    yield f"cases/{name}", path
    programs = os.path.join(root, "programs")
    for name in sorted(os.listdir(programs)):
        path = os.path.join(programs, name)
        if os.path.isdir(path):
            yield f"programs/{name}", path


def ledger(path):
    folder = path if os.path.isdir(path) else os.path.dirname(path)
    return os.path.join(folder, ".accord")


def once(path):
    shutil.rmtree(ledger(path), ignore_errors=True)
    start = time.perf_counter()
    done = subprocess.run([accord, "check", path, "--json"], capture_output=True, text=True)
    elapsed = time.perf_counter() - start
    if done.returncode == 4 and json.loads(done.stdout)["status"] == "rejected":
        return elapsed, None
    if done.returncode not in (0, 1, 3):
        sys.exit(f"{path}: exit {done.returncode}\n{done.stdout}\n{done.stderr}")
    return elapsed, json.loads(done.stdout)


with tempfile.TemporaryDirectory() as work:
    root = os.path.join(work, "conformance")
    shutil.copytree(conformance, root, ignore=shutil.ignore_patterns(".accord"))
    summary = {}
    for name, path in targets(root):
        _, report = once(path)
        if report is None:
            summary[name] = "rejected"
            continue
        times = [once(path)[0] for _ in range(runs)]
        outcomes = collections.Counter(
            o["outcome"] for d in report["definitions"] for o in d["obligations"]
        )
        summary[name] = {
            "median_s": round(statistics.median(times), 3),
            "times_s": [round(t, 3) for t in times],
            "obligations": sum(outcomes.values()),
            "outcomes": dict(sorted(outcomes.items())),
        }
print(json.dumps(summary, indent=1))
