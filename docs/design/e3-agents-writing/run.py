"""Run E3's agents: each task, in each arm, with each model, in a fresh isolated folder.

    python run.py --models haiku,sonnet --reps 2 [--tasks t01-clamp,...] [--arms accord,lemmascript] [--out DIR]

Runs are resumable: a run whose result.json exists is skipped. Each run uses
Claude Code in print mode, signed in with the operator's subscription,
restricted to the task folder and to the checker wrapper, with no user or
project instructions, memory or MCP servers.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from e3 import ARMS, HERE, MAX_CHECKS, TASKS, task_page

RUNS = HERE / "runs"
RUN_TIMEOUT = 1200
MAX_ATTEMPTS = 3


def prompt(arm: str) -> str:
    return (HERE / "prompt.txt").read_text(encoding="utf-8").format(
        language=ARMS[arm]["language"], file="answer" + ARMS[arm]["ext"],
        check=f"python {(HERE / 'check.py').as_posix()}", checks=MAX_CHECKS)


def scrub(text: str, work: Path) -> str:
    """Replace the operator's local paths with placeholders, so runs can be published."""
    for path, name in ((work, "<task folder>"), (HERE, "<e3>"), (Path(tempfile.gettempdir()), "<temp>"),
                       (Path.home(), "<home>")):
        posix = path.as_posix()
        git_bash = re.sub(r"^([A-Za-z]):", lambda m: "/" + m.group(1).lower(), posix)
        for form in {str(path), posix, git_bash, str(path).replace("\\", "\\\\")}:
            text = re.sub(re.escape(form), lambda _: name, text, flags=re.IGNORECASE)
    return re.sub(rf"\b{re.escape(Path.home().name)}\b", "<user>", text, flags=re.IGNORECASE)


def run_one(task: dict, arm: str, model: str, rep: int, runs: Path) -> None:
    run_id = f"{task['id']}__{arm}__{model}__r{rep}"
    out = runs / run_id
    attempt = 1
    if (out / "result.json").exists():
        previous = json.loads((out / "result.json").read_text(encoding="utf-8"))
        if not previous["invalid"] or previous["attempt"] >= MAX_ATTEMPTS:
            return
        attempt = previous["attempt"] + 1
        with open(runs / "invalid.jsonl", "a", encoding="utf-8", newline="\n") as invalid:
            invalid.write(json.dumps({"run": run_id, **previous}) + "\n")
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    work = Path(tempfile.mkdtemp(prefix="e3-work-"))
    (work / "task.md").write_text(task_page(task, arm), encoding="utf-8", newline="\n")
    shutil.copyfile(HERE / "reference" / ARMS[arm]["page"], work / "reference.md")
    check_rule = f"Bash(python {(HERE / 'check.py').as_posix()} *)"
    cmd = ["claude", "-p", prompt(arm), "--model", model, "--restricted", "--strict-mcp-config",
           "--tools", "Read,Write,Edit,Bash", "--allowedTools", "Read", "Write", "Edit", check_rule,
           "--permission-mode", "acceptEdits", "--no-session-persistence",
           "--output-format", "stream-json", "--verbose"]
    env = dict(os.environ, E3_RUN_DIR=str(out), E3_TASK=task["id"])
    started = time.monotonic()
    timed_out = False
    with open(out / "transcript.jsonl", "w", encoding="utf-8") as transcript:
        try:
            proc = subprocess.run(cmd, cwd=work, env=env, stdout=transcript, stderr=subprocess.PIPE,
                                  text=True, encoding="utf-8", errors="replace", timeout=RUN_TIMEOUT)
            exit_code, stderr = proc.returncode, proc.stderr
        except subprocess.TimeoutExpired:
            exit_code, stderr, timed_out = None, "", True
    answer = work / ("answer" + ARMS[arm]["ext"])
    if answer.exists():
        shutil.copyfile(answer, out / answer.name)
    shutil.rmtree(work, ignore_errors=True)
    text = scrub((out / "transcript.jsonl").read_text(encoding="utf-8"), work)
    (out / "transcript.jsonl").write_text(text, encoding="utf-8", newline="\n")
    final, init, tool_calls = {}, {}, 0
    for line in text.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") == "assistant":
            tool_calls += sum(c.get("type") == "tool_use" for c in event["message"].get("content", []))
        if event.get("type") == "result":
            final = event
        if event.get("type") == "system" and event.get("subtype") == "init":
            init = event
    log = out / "checks.jsonl"
    errored = exit_code != 0 or final.get("is_error") is not False
    result = {
        "task": task["id"], "arm": arm, "model": model, "rep": rep, "attempt": attempt,
        "invalid": errored and not timed_out and tool_calls == 0, "tool_calls": tool_calls,
        "model_id": init.get("model"), "auth": init.get("apiKeySource"),
        "exit": exit_code, "timed_out": timed_out, "seconds": round(time.monotonic() - started, 1),
        "checks_used": len(log.read_text(encoding="utf-8").splitlines()) if log.exists() else 0,
        "turns": final.get("num_turns"), "output_tokens": (final.get("usage") or {}).get("output_tokens"),
        "is_error": final.get("is_error"), "denials": len(final.get("permission_denials") or []),
        "stderr": scrub(stderr[-2000:], work),
    }
    (out / "result.json").write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8", newline="\n")
    print(run_id, "checks:", result["checks_used"], "seconds:", result["seconds"], flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--models", required=True)
    parser.add_argument("--reps", type=int, required=True)
    parser.add_argument("--tasks", default=",".join(t["id"] for t in TASKS))
    parser.add_argument("--arms", default=",".join(ARMS))
    parser.add_argument("--out", type=Path, default=RUNS, help="where runs go (default: runs/)")
    args = parser.parse_args()
    chosen = set(args.tasks.split(","))
    unknown = chosen - {t["id"] for t in TASKS}
    if unknown or not set(args.arms.split(",")) <= set(ARMS):
        sys.exit(f"unknown task or arm: {sorted(unknown)} {args.arms}")
    for rep in range(1, args.reps + 1):
        for model in args.models.split(","):
            for task in [t for t in TASKS if t["id"] in chosen]:
                for arm in args.arms.split(","):
                    run_one(task, arm, model, rep, args.out.resolve())


if __name__ == "__main__":
    main()
