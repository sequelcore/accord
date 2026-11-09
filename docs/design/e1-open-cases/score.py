"""Score E1: check every claimed open case with Accord, then apply the pass mark.

A claim (an input and two results) is valid when, for each result, a function
with the corpus file's `requires` and `ensures`, a `requires` pinning each
parameter to the claimed input, and the result as its body, has every
obligation proved; an example calling it on that input passes, so the input
is one the contract admits; and the two results differ.

Usage: python score.py <accord binary>   (with ACCORD_Z3 and ACCORD_CVC5 set)

Writes scores.csv beside this file and prints the measures.
"""

import csv
import json
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).parent
ACCORD = sys.argv[1]
VALUE = re.compile(r"^(-?\d+|true|false|[A-Z]\w*\.[A-Z]\w*)$")
NAME = re.compile(r"^[a-z_]\w*$")


def literal(value) -> str | None:
    """A JSON value as Accord source, or None if it isn't a plain value."""
    if isinstance(value, bool):
        text = "true" if value else "false"
    elif isinstance(value, int):
        text = str(value)
    elif isinstance(value, str):
        text = value.strip()
    else:
        return None
    return text if VALUE.match(text) else None


def contract(source: str) -> dict:
    """The declarations before the function, its parameters and return type
    as written, and its `requires` and `ensures` lines."""
    lines = source.splitlines()
    start = next(i for i, l in enumerate(lines) if re.match(r"^(pub )?fn ", l))
    end = next(i for i in range(start, len(lines)) if "->" in lines[i])
    header = " ".join(l.strip() for l in lines[start:end + 1])
    params_text = header[header.index("(") + 1:header.rindex(")")]
    params = [p.split(":")[0].strip() for p in params_text.split(",") if p.strip()]
    ret = header.split("->")[1].strip()
    clauses, i = [], end + 1
    while i < len(lines) and re.match(r"^\s+(requires|ensures|intent)\b", lines[i]):
        if not lines[i].strip().startswith("intent"):
            clauses.append(lines[i].strip())
        i += 1
    return {
        "before": "\n".join(lines[:start]),
        "params_text": params_text.strip().rstrip(","),
        "params": params,
        "ret": ret,
        "requires": [c for c in clauses if c.startswith("requires")],
        "ensures": [c for c in clauses if c.startswith("ensures")],
    }


def meets(c: dict, inputs: dict, result: str) -> bool:
    pins = [f"requires {name} == {value}" for name, value in inputs.items()]
    call = ", ".join(f"{name}: {inputs[name]}" for name in c["params"])
    source = "\n".join([
        c["before"],
        f"fn claim({c['params_text']}) -> {c['ret']}",
        *(f"    {line}" for line in c["requires"] + pins + c["ensures"]),
        "{",
        f"    {result}",
        "}",
        "",
        f"example claim({call}) == {result}",
        "",
    ])
    with tempfile.TemporaryDirectory() as folder:
        path = pathlib.Path(folder) / "claim.accord"
        path.write_text(source, encoding="utf-8")
        done = subprocess.run([ACCORD, "check", str(path), "--json"], capture_output=True, text=True)
    if done.returncode == 4 or not done.stdout.strip():
        return False
    receipt = json.loads(done.stdout)
    (definition,) = receipt["definitions"]
    obligations = definition["obligations"]
    examples = definition["examples"]
    return (
        all(o["outcome"] == "proved" for o in obligations)
        and len(examples) == 1
        and examples[0]["run"] == "passed"
    )


def claims(reply: str) -> list | None:
    """The reply's claimed open cases, or None if it can't be read."""
    start, end = reply.find("{"), reply.rfind("}")
    if start < 0 or end < start:
        return None
    try:
        cases = json.loads(reply[start:end + 1]).get("open_cases")
    except (json.JSONDecodeError, AttributeError):
        return None
    return cases if isinstance(cases, list) else None


def valid(c: dict, claim) -> bool:
    if not isinstance(claim, dict):
        return False
    raw_inputs, results = claim.get("inputs"), claim.get("results")
    if not isinstance(raw_inputs, dict) or not isinstance(results, list) or len(results) != 2:
        return False
    inputs = {}
    for name, value in raw_inputs.items():
        text = literal(value)
        if not NAME.match(str(name)) or text is None:
            return False
        inputs[name] = text
    if sorted(inputs) != sorted(c["params"]):
        return False
    first, second = literal(results[0]), literal(results[1])
    if first is None or second is None or first == second:
        return False
    return meets(c, inputs, first) and meets(c, inputs, second)


def main() -> None:
    rows = []
    for line in (HERE / "replies.jsonl").read_text(encoding="utf-8").splitlines():
        record = json.loads(line)
        c = contract((HERE / "corpus" / f"{record['file']}.accord").read_text(encoding="utf-8"))
        cases = claims(record["reply"])
        checked = [valid(c, claim) for claim in (cases or [])]
        rows.append({
            "model": record["model"],
            "file": record["file"],
            "repetition": record["repetition"],
            "readable": cases is not None,
            "claims": len(checked),
            "valid": sum(checked),
            "invalid": len(checked) - sum(checked),
        })
    with open(HERE / "scores.csv", "w", encoding="utf-8", newline="") as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    planted = [r for r in rows if r["file"].startswith("a")]
    clean = [r for r in rows if r["file"].startswith("c")]
    found = [r for r in planted if r["valid"] > 0]
    all_claims = sum(r["claims"] for r in rows)
    print(f"agent recall: {len(found)}/{len(planted)}")
    for file in sorted({r["file"] for r in planted}):
        on = [r for r in planted if r["file"] == file]
        print(f"  {file}: {sum(r['valid'] > 0 for r in on)}/{len(on)}")
    print(f"agent precision: {sum(r['valid'] for r in rows)}/{all_claims} claims valid")
    print(f"  on determined files: {sum(r['valid'] for r in clean)}/{sum(r['claims'] for r in clean)}")
    agent_only = sum(r["valid"] for r in clean)
    missed = [
        f for f in sorted({r["file"] for r in planted})
        if sum(r["valid"] > 0 for r in planted if r["file"] == f) * 2
        <= len([r for r in planted if r["file"] == f])
    ]
    print(f"agent-only finds: {agent_only}")
    print(f"planted files at least half the conversations missed: {missed}")
    passed = agent_only == 0 and bool(missed)
    print("pass mark:", "met" if passed else "not met")


if __name__ == "__main__":
    main()
