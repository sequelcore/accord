"""Score E3's runs, or check the scorer and the tasks with --self-test.

    python score.py --self-test
    python score.py [runs folder, default runs/]

Needs ACCORD_E3_ACCORD and ACCORD_E3_LSC (see README.md), Node 22.6 or later,
and, for Accord, the solvers in ACCORD_Z3 and ACCORD_CVC5.
"""

import csv
import json
import random
import sys
import tempfile
from pathlib import Path

from e3 import ARMS, HERE, TASKS, check, hidden

RUNS = HERE / "runs"
MARGIN = 0.10


def solution(task: dict, arm: str) -> Path:
    return HERE / "solutions" / arm / (task["id"].split("-", 1)[1] + ARMS[arm]["ext"])


def self_test() -> None:
    failures = []
    for task in TASKS:
        for arm in ARMS:
            with tempfile.TemporaryDirectory(prefix="e3-self-") as tmp:
                path = Path(tmp) / solution(task, arm).name
                path.write_bytes(solution(task, arm).read_bytes())
                accepted = check(path, task, arm)["accepted"]
                passed = hidden(path, task, arm)
            if not accepted or not all(passed):
                failures.append(f"{task['id']} {arm}: accepted={accepted}, hidden {sum(passed)}/{len(passed)}")
    wrong = {
        "accord": "fn clamp(value: i64, low: i64, high: i64) -> i64\n    requires low <= high\n{\n    value\n}\n",
        "lemmascript": "//@ requires low <= high\nexport function clamp(value: number, low: number, high: number): number {\n  return value;\n}\n",
    }
    for arm, source in wrong.items():
        with tempfile.TemporaryDirectory(prefix="e3-self-") as tmp:
            path = Path(tmp) / ("clamp" + ARMS[arm]["ext"])
            path.write_text(source, encoding="utf-8")
            passed = hidden(path, TASKS[0], arm)
        if all(passed) or not any(passed):
            failures.append(f"wrong clamp {arm}: hidden {sum(passed)}/{len(passed)}, expected some but not all")
    if failures:
        raise SystemExit("FAIL:\n" + "\n".join(failures))
    examples = sum(len(t["examples"]) for t in TASKS)
    print(f"PASS: {len(TASKS)} tasks, both reference solutions accepted with all {examples} hidden examples; "
          "a wrong solution fails some hidden examples in each arm")


def interval(pairs: list[tuple[int, int]], seed: int = 3) -> tuple[float, float]:
    """A 95% paired bootstrap interval for Accord's success rate minus LemmaScript's."""
    rng = random.Random(seed)
    diffs = []
    for _ in range(10000):
        sample = [rng.choice(pairs) for _ in pairs]
        diffs.append(sum(a - b for a, b in sample) / len(sample))
    diffs.sort()
    return diffs[249], diffs[9749]


def score(runs: Path) -> None:
    rows = []
    tasks = {t["id"]: t for t in TASKS}
    for result in sorted(runs.glob("*/result.json")):
        r = json.loads(result.read_text(encoding="utf-8"))
        task = tasks[r["task"]]
        answer = result.parent / ("answer" + ARMS[r["arm"]]["ext"])
        if answer.exists():
            with tempfile.TemporaryDirectory(prefix="e3-score-") as tmp:
                copy = Path(tmp) / answer.name
                copy.write_bytes(answer.read_bytes())
                verdict = check(copy, task, r["arm"])
                passed = hidden(copy, task, r["arm"])
        else:
            verdict, passed = {"accepted": False, "determined": None}, [False] * len(task["examples"])
        rows.append({
            "run": result.parent.name, "task": r["task"], "arm": r["arm"], "model": r["model"], "rep": r["rep"],
            "checks_used": r["checks_used"], "accepted": verdict["accepted"],
            "hidden_passed": sum(passed), "hidden_total": len(passed),
            "success": verdict["accepted"] and all(passed),
            "failure": "" if verdict["accepted"] and all(passed) else
                       "no final file" if not answer.exists() else
                       "not accepted" if not verdict["accepted"] else "hidden example",
            "invalid": r["invalid"], "denials": r["denials"],
            "determined": "" if verdict["determined"] is None else verdict["determined"],
            "timed_out": r["timed_out"], "output_tokens": r.get("output_tokens", ""),
        })
    if not rows:
        raise SystemExit("no runs to score")
    with open(runs / "scores.csv", "w", encoding="utf-8", newline="") as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    def rate(subset):
        return sum(r["success"] for r in subset), len(subset)

    for arm in ARMS:
        s, n = rate([r for r in rows if r["arm"] == arm])
        print(f"{arm}: {s}/{n} verified successes")
        for model in sorted({r["model"] for r in rows}):
            s, n = rate([r for r in rows if r["arm"] == arm and r["model"] == model])
            print(f"  {model}: {s}/{n}")
    for task in TASKS:
        cells = []
        for arm in ARMS:
            s, n = rate([r for r in rows if r["arm"] == arm and r["task"] == task["id"]])
            cells.append(f"{arm} {s}/{n}")
        print(f"{task['id']}: " + ", ".join(cells))
    keyed = {(r["task"], r["model"], r["rep"], r["arm"]): r["success"] for r in rows}
    pairs = [(int(keyed[(t, m, p, "accord")]), int(keyed[(t, m, p, "lemmascript")]))
             for (t, m, p, arm) in keyed if arm == "accord" and (t, m, p, "lemmascript") in keyed]
    if not pairs:
        raise SystemExit("no paired runs")
    diff = sum(a - b for a, b in pairs) / len(pairs)
    low, high = interval(pairs)
    print(f"paired runs: {len(pairs)}; Accord only: {sum(a > b for a, b in pairs)}, "
          f"LemmaScript only: {sum(b > a for a, b in pairs)}")
    print(f"difference (Accord - LemmaScript): {diff:+.3f}, 95% paired bootstrap interval [{low:+.3f}, {high:+.3f}]")
    for kind in ("no final file", "not accepted", "hidden example"):
        print(f"failures, {kind}: " + ", ".join(f"{arm} {sum(r['failure'] == kind and r['arm'] == arm for r in rows)}" for arm in ARMS))
    flagged = [r["run"] for r in rows if r["invalid"] or r["denials"]]
    print("still invalid or with a permission denial:", flagged or "none")
    print("pass mark:", "met" if diff >= -MARGIN else "not met")


if __name__ == "__main__":
    self_test() if sys.argv[1:] == ["--self-test"] else score(Path(sys.argv[1]) if sys.argv[1:] else RUNS)
