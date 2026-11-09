"""What E3's checker wrapper, harness and scorer share: tasks, rendering, acceptance and hidden examples."""

import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = json.loads((HERE / "tasks.json").read_text(encoding="utf-8"))["tasks"]
MAX_CHECKS = 5
CHECK_TIMEOUT = 300

ARMS = {
    "accord": {"ext": ".accord", "language": "Accord", "page": "accord.md"},
    "lemmascript": {"ext": ".ts", "language": "TypeScript with LemmaScript", "page": "lemmascript.md"},
}
ACCORD_TYPES = {"Int": "i64", "Bool": "Bool", "Text": "Text", "List<Text>": "List<Text>"}
TS_TYPES = {"Int": "number", "Bool": "boolean", "Text": "string", "List<Text>": "readonly string[]"}


def tool(name: str) -> str:
    """The path of a tool the run needs, from its environment variable."""
    variable = {"accord": "ACCORD_E3_ACCORD", "lsc": "ACCORD_E3_LSC"}[name]
    path = os.environ.get(variable)
    if not path or not Path(path).exists():
        raise SystemExit(f"set {variable} to the {name} executable")
    return path


def snake(variant: str) -> str:
    """A variant's TypeScript literal: `TimedOut` is `timed_out`."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", variant).lower()


def variants(task: dict) -> set[str]:
    """Every enum variant the task declares."""
    found = set()
    for decl in task["accord"]["decls"]:
        body = re.search(r"\{([^}]*)\}", decl)
        found.update(v.strip() for v in body.group(1).split(","))
    return found


def ts_name(task: dict, param: str) -> str:
    return task["ts"]["params"][task["params"].index(param)]


def signature(task: dict, arm: str) -> str:
    if arm == "accord":
        params = ", ".join(f"{p}: {ACCORD_TYPES.get(t, t)}" for p, t in zip(task["params"], task["types"]))
        return f"fn {task['accord']['name']}({params}) -> {ACCORD_TYPES.get(task['result'], task['result'])}"
    params = ", ".join(f"{p}: {TS_TYPES.get(t, t)}" for p, t in zip(task["ts"]["params"], task["types"]))
    return f"export function {task['ts']['name']}({params}): {TS_TYPES.get(task['result'], task['result'])}"


def prose(task: dict, arm: str) -> str:
    """The task's prose, naming parameters and values as the arm's language does."""
    text = task["prose"]
    if arm == "accord":
        return text
    for param in task["params"]:
        text = text.replace(f"`{param}`", f"`{ts_name(task, param)}`")
    names = "|".join(sorted(variants(task), key=len, reverse=True))
    if names:
        text = re.sub(rf"\b({names})\b", lambda m: f'"{snake(m.group(1))}"', text)
    return text


def task_page(task: dict, arm: str) -> str:
    lang = "accord" if arm == "accord" else "ts"
    page = f"# Task\n\n{prose(task, arm)}\n\n## Signature\n\n```{lang}\n{signature(task, arm)}\n```\n"
    decls = task[lang]["decls"]
    if decls:
        page += f"\n## Declarations to include\n\n```{lang}\n" + "\n".join(decls) + "\n```\n"
    return page


def accord_value(value, kind: str) -> str:
    if kind == "Bool":
        return "true" if value else "false"
    if kind == "Int":
        return str(value)
    if kind == "Text":
        return json.dumps(value)
    if kind == "List<Text>":
        return "[" + ", ".join(json.dumps(v) for v in value) + "]"
    return value


def ts_value(value, kind: str):
    if kind in ("Bool", "Int", "Text", "List<Text>"):
        return value
    return snake(value)


def accord_example(task: dict, example: dict) -> str:
    args = [accord_value(example["args"][p], t) for p, t in zip(task["params"], task["types"])]
    if len(args) == 1:
        call = f"{task['accord']['name']}({args[0]})"
    else:
        call = f"{task['accord']['name']}(" + ", ".join(f"{p}: {a}" for p, a in zip(task["params"], args)) + ")"
    return f"example {call} == {accord_value(example['expected'], task['result'])}"


def run(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                              errors="replace", timeout=CHECK_TIMEOUT)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, 124, "", f"timed out after {CHECK_TIMEOUT} s")


def accord_receipt(path: Path) -> tuple[dict | None, str]:
    proc = run([tool("accord"), "check", path.name, "--json"], path.parent)
    try:
        return json.loads(proc.stdout), proc.stdout
    except json.JSONDecodeError:
        return None, proc.stdout + proc.stderr


def check_accord(path: Path, task: dict) -> dict:
    """Accepted: the task's function is present, and every obligation is proved and every example passes."""
    text = run([tool("accord"), "check", path.name], path.parent)
    receipt, raw = accord_receipt(path)
    definitions = (receipt or {}).get("definitions", [])
    function = next((d for d in definitions if d.get("name") == task["accord"]["name"]), None)
    accepted = function is not None and all(
        o.get("outcome") == "proved" for d in definitions for o in d.get("obligations", [])
    ) and all(e.get("run") == "passed" for d in definitions for e in d.get("examples", []))
    determined = bool(function and (function.get("openness") or {}).get("determined"))
    return {"accepted": accepted, "determined": determined, "output": text.stdout + text.stderr}


def check_lemmascript(path: Path, task: dict) -> dict:
    """Accepted: the task's function is exported, and Dafny verified at least one obligation with no errors."""
    for generated in (path.with_suffix(".dfy"), path.with_suffix(".dfy.gen")):
        generated.unlink(missing_ok=True)
    gen = run([tool("lsc"), "gen", "--backend=dafny", path.name], path.parent)
    output = gen.stdout + gen.stderr
    if gen.returncode != 0:
        return {"accepted": False, "determined": None, "output": output}
    check = run([tool("lsc"), "check", "--backend=dafny", path.name], path.parent)
    output += check.stdout + check.stderr
    verified = re.search(r"finished with (\d+) verified, 0 errors", check.stdout)
    exported = re.search(rf"export function {re.escape(task['ts']['name'])}\s*\(", path.read_text(encoding="utf-8"))
    accepted = check.returncode == 0 and bool(verified) and int(verified.group(1)) >= 1 and bool(exported)
    return {"accepted": accepted, "determined": None, "output": output}


def check(path: Path, task: dict, arm: str) -> dict:
    return (check_accord if arm == "accord" else check_lemmascript)(path, task)


def hidden_accord(path: Path, task: dict) -> list[bool]:
    """Each hidden example's result, run by Accord's interpreter on a copy of the file."""
    with tempfile.TemporaryDirectory(prefix="e3-hidden-") as tmp:
        copy = Path(tmp) / "hidden.accord"
        source = path.read_text(encoding="utf-8").rstrip("\n") + "\n\n"
        first = source.count("\n") + 1
        lines = [accord_example(task, e) for e in task["examples"]]
        copy.write_text(source + "\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        receipt, _ = accord_receipt(copy)
    runs = {e["line"]: e["run"] for d in (receipt or {}).get("definitions", []) for e in d.get("examples", [])}
    return [runs.get(first + i) == "passed" for i in range(len(lines))]


RUNNER = """
import { readFileSync } from "node:fs";
const mod = await import("./solution.ts");
const cases = JSON.parse(readFileSync("cases.json", "utf8"));
const fn = mod[cases.name];
const out = cases.examples.map(([args, expected]) => {
  try { return typeof fn === "function" && JSON.stringify(fn(...args)) === JSON.stringify(expected); }
  catch { return false; }
});
console.log(JSON.stringify(out));
"""


def hidden_lemmascript(path: Path, task: dict) -> list[bool]:
    """Each hidden example's result, run by Node on a copy of the file."""
    examples = [[[ts_value(e["args"][p], t) for p, t in zip(task["params"], task["types"])],
                 ts_value(e["expected"], task["result"])] for e in task["examples"]]
    with tempfile.TemporaryDirectory(prefix="e3-hidden-") as tmp:
        shutil.copyfile(path, Path(tmp) / "solution.ts")
        (Path(tmp) / "cases.json").write_text(json.dumps({"name": task["ts"]["name"], "examples": examples}), encoding="utf-8")
        (Path(tmp) / "runner.mjs").write_text(RUNNER, encoding="utf-8")
        proc = run(["node", "--no-warnings", "--experimental-strip-types", "runner.mjs"], Path(tmp))
    try:
        results = json.loads(proc.stdout.strip().splitlines()[-1])
        return [bool(r) for r in results]
    except (IndexError, json.JSONDecodeError):
        return [False] * len(examples)


def hidden(path: Path, task: dict, arm: str) -> list[bool]:
    return (hidden_accord if arm == "accord" else hidden_lemmascript)(path, task)
