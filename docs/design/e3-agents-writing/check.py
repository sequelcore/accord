"""The only command an E3 agent may run: check its file, at most MAX_CHECKS times.

    python check.py <file>

The harness sets E3_RUN_DIR and E3_TASK. Each check is logged to the run's
checks.jsonl, which also counts the checks used.
"""

import json
import os
import sys
from pathlib import Path

from e3 import ARMS, MAX_CHECKS, TASKS, check


def main() -> int:
    run_dir, task_id = Path(os.environ["E3_RUN_DIR"]), os.environ["E3_TASK"]
    log = run_dir / "checks.jsonl"
    used = len(log.read_text(encoding="utf-8").splitlines()) if log.exists() else 0
    if len(sys.argv) != 2:
        print("usage: python check.py <file>")
        return 2
    if used >= MAX_CHECKS:
        print(f"No checks left: all {MAX_CHECKS} were used.")
        return 2
    path = Path(sys.argv[1]).resolve()
    arm = next((a for a, spec in ARMS.items() if path.suffix == spec["ext"]), None)
    if arm is None or not path.exists():
        print(f"no such file to check: {sys.argv[1]}")
        return 2
    task = next(t for t in TASKS if t["id"] == task_id)
    verdict = check(path, task, arm)
    with log.open("a", encoding="utf-8") as out:
        out.write(json.dumps({"check": used + 1, "accepted": verdict["accepted"]}) + "\n")
    print(verdict["output"].rstrip())
    print()
    print("Result: accepted" if verdict["accepted"] else "Result: not accepted")
    print(f"Checks left: {MAX_CHECKS - used - 1}")
    return 0 if verdict["accepted"] else 1


if __name__ == "__main__":
    sys.exit(main())
