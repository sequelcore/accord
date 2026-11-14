"""Build/check E2 draft cards and the owner's code-free readability packets.

Usage: python materials.py <accord binary> [--write]
Build the binary first. Without --write, stale generated files are errors.
This validates materials, never recruits or records participant responses.
"""
import argparse
import csv
import hashlib
import io
import json
import os
import pathlib
import random
import re
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
KEY = json.loads((HERE / "answer-key.json").read_text(encoding="utf-8"))
KNOWN_CODES = {f"ACC{i:04}" for i in range(1, 13)}
if not __debug__:
    raise RuntimeError("material validation requires assertions enabled; omit -O")
REQUIREMENTS = {
    "gate": """Review a verdict for one file's proof. Inputs are its outcome, a failure text,
and the failure texts already sent back. Text membership uses exact equality.
Passed must give Proved. Failed or Vacuous gives SendBack for a new failure and
NoProgress for an already-sent failure. TimedOut, Cancelled, and NotStarted must
give Operator. All enum outcomes and all texts/lists are admitted. The result
is advisory; this pure function does not itself retry, stop, or contact anyone.""",
    "cedar": """Review a partial authorization decision. Each Boolean says whether there is
at least one satisfied forbid, satisfied permit, residual permit, or residual
forbid. Residual means the policy still depends on unknown information. All 16
Boolean combinations are admitted. A satisfied forbid always denies. With no
satisfied or residual permit, deny even if a residual forbid exists. Otherwise
a residual forbid leaves the answer Unknown. Without forbids, a satisfied
permit allows; a residual permit alone leaves Unknown. Unknown grants no access
and is an authorization result, not an undecided solver outcome. Evaluation
and assignment of policies to these collections are outside this abstraction.""",
}
QUESTIONS = {
    "gate": """1. With changed=false and selected=true, is verification required? What about both false?
2. Can ls change files under this rule? Can an unlisted tool when neither native_question nor read_only holds?
3. What do Failed with a new failure and Failed with a repeated failure return?
4. What do Passed, TimedOut, and NotStarted return from gate_verdict?
5. What do preexisting Failed and preexisting Vacuous with a new failure return from command_verdict?
6. Does keeps_working continue for [Proved, Operator]? For [Proved, SendBack]?
7. Does this prove that a Passed input is a sound proof or that the whole checked program is verified? Explain.
8. Does determined plus verified establish that the specification captures the intended workflow? Explain.""",
    "cedar": """1. What does each of the four inputs mean, and which combinations are admitted?
2. With every flag true, what is the result?
3. With no satisfied or residual permit but a residual forbid, what is the result?
4. With a satisfied permit and residual forbid but no satisfied forbid, what is the result?
5. With only a residual permit, what is the result?
6. With a satisfied permit, a residual permit, and no forbids, what is the result?
7. Does Unknown mean the solvers did not settle? Does it make this function's contract open?
8. Does this prove that policies were evaluated into the right collections, or that Unknown should grant access? Explain.""",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check(binary, path):
    args = [str(binary), "check", path]
    done = subprocess.run(args + ["--json"], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=120)
    assert done.returncode in (0, 1, 3), (path, done.returncode, done.stderr)
    report = json.loads(done.stdout)
    validate_wire(report)
    text = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=120)
    assert text.returncode == done.returncode, path
    # Saved cards omit volatile cache-use markers, not proof outcomes or limits.
    return report, text.stdout.replace("  [reused]", "").rstrip() + "\n"


def validate_wire(report):
    assert report["receipt"] == 5 and report["status"] == "checked", "unknown receipt contract"
    for finding in report["diagnostics"]:
        assert finding["code"] in KNOWN_CODES, "unknown diagnostic code"
        assert report_pointer(report, finding["evidence"]) is not None

    def values(node):
        if isinstance(node, dict):
            if "source" in node and "valid" in node:
                assert node["valid"] is True, "unusable witness decoding"
            for value in node.values():
                values(value)
        elif isinstance(node, list):
            for value in node:
                values(value)
    values(report)


def spec(d):
    types = "\n".join(f"enum {t['name']} {{ {', '.join(t['variants'])} }}" for t in d["types"])
    intent = "" if d["intent"] is None else f"\n    intent {json.dumps(d['intent'])}"
    clauses = "\n".join("    " + c["text"] for c in d["specification"])
    return f"{types}\n\n{d['signature']}{intent}\n{clauses}\n"


def code_only(source, d):
    start = re.search(r"^(decide\s*\{|\{)", source, re.M)
    assert start, "corpus has one closed-form function body"
    end = source.index("\n}", start.start()) + 2
    types = "\n".join(f"enum {t['name']} {{ {', '.join(t['variants'])} }}" for t in d["types"])
    return f"{types}\n\n{d['signature']}\n{source[start.start():end]}\n"


def cedar_attribution(artifact):
    parent = (HERE / artifact).parent
    license_path = pathlib.Path(os.path.relpath(ROOT / "LICENSE-APACHE", parent)).as_posix()
    notice_path = pathlib.Path(os.path.relpath(ROOT / "conformance/programs/cedar/NOTICE", parent)).as_posix()
    return f"Copyright Cedar Contributors. Modified Boolean port of Cedar's partial\nauthorization decision, licensed under [Apache-2.0]({license_path});\nsee the retained [notice]({notice_path}).\n\n"


def allocation():
    slots = []
    for arm in ("code", "specification", "review"):
        for i in range(30):
            tasks = [f"R{i % 6 + 1:02}", f"R{(i % 6 + i // 6) % 6 + 7:02}"]
            if i % 2:
                tasks.reverse()
            slots.append(dict(arm=arm, stratum="experienced" if i < 15 else "less_experienced", first=tasks[0], second=tasks[1]))
    random.Random(4607).shuffle(slots)
    out = io.StringIO(newline="")
    writer = csv.DictWriter(out, fieldnames=["reviewer", "arm", "stratum", "first", "second"], lineterminator="\n")
    writer.writeheader()
    for i, slot in enumerate(slots, 1):
        writer.writerow(dict(reviewer=f"P{i:03}", **slot))
    rows = list(csv.DictReader(io.StringIO(out.getvalue())))
    assert len(rows) == 90 and len({r["reviewer"] for r in rows}) == 90
    for arm in ("code", "specification", "review"):
        chosen = [r for r in rows if r["arm"] == arm]
        assert len(chosen) == 30
        assert sum(r["stratum"] == "experienced" for r in chosen) == 15
        assert sum(int(r["first"][1:]) <= 6 for r in chosen) == 15
        for task in KEY:
            assert sum(task["task"] in (r["first"], r["second"]) for r in chosen) == 5
        assert all((int(r["first"][1:]) <= 6) != (int(r["second"][1:]) <= 6) for r in chosen)
    return out.getvalue()


def validate_category(task, report):
    (d,) = report["definitions"]
    assert report["standing"] == task["standing"], (task["task"], report["standing"])
    o = d["openness"]
    assert ("open" if "open" in o else "determined" if o.get("determined") else "undecided") == task["openness"], task
    assert len(d["examples"]) == task["examples"]
    assert all(e["run"] == "passed" for e in d["examples"]), task
    refuted = [o for o in d["obligations"] if o["outcome"] == "refuted"]
    assert bool(refuted) == (task["category"] == "body"), task
    assert all(o["outcome"] in ("proved", "refuted") for o in d["obligations"]), task
    if task["category"] == "body":
        assert d["strength"] is None, "a refuted baseline has no contract-strength claim"
    elif task["category"] in ("weak_specification", "misdeclared_freedom"):
        assert d["strength"]["survivors"], task
    else:
        assert not d["strength"]["survivors"], task
    assert bool(o["free"]) == (task["category"] == "misdeclared_freedom"), task
    for finding in report["diagnostics"]:
        assert report_pointer(report, finding["evidence"]) is not None
    return d


def report_pointer(document, pointer):
    value = document
    for part in pointer.split("/")[1:]:
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def literal(value, ty):
    if ty["kind"] == "enum":
        return ty["name"] + "." + value
    if ty["kind"] == "list":
        return "[" + ", ".join(literal(v, ty["element"]) for v in value) + "]"
    if ty["kind"] == "bool":
        return str(value).lower()
    if ty["kind"] == "text":
        assert '"' not in value and "\n" not in value, "current Accord literals have no escapes"
        return '"' + value + '"'
    return str(value)


def validate_witness(binary, task, source, d):
    if task["category"] == "clean":
        return
    values = {p["name"]: literal(task["witness"][p["name"]], p["type"]) for p in d["parameters"]}
    args = ", ".join(name + ": " + value for name, value in values.items())
    call = f"example {d['name']}({args}) == {task['required_result']}\n"
    with tempfile.TemporaryDirectory(prefix="accord-e2-witness-") as folder:
        path = pathlib.Path(folder) / "witness.accord"
        reference_id = "R01" if task["domain"] == "gate" else "R07"
        reference = (HERE / "corpus" / f"{reference_id}.accord").read_text(encoding="utf-8")
        path.write_text(reference + call, encoding="utf-8", newline="\n")
        reference_report, _ = check(binary, str(path))
        assert reference_report["definitions"][0]["examples"][-1]["run"] == "passed", task
        if task["category"] in ("body", "wrong_complete", "excluded_input"):
            path.write_text(source + call, encoding="utf-8", newline="\n")
            candidate, _ = check(binary, str(path))
            assert candidate["definitions"][0]["examples"][-1]["run"] == "failed", task
        else:
            # Pin an admitted witness and return the forbidden-by-requirement
            # alternative. This must still meet the weakened candidate contract.
            alternative = "Proved" if task["domain"] == "gate" else "Allow"
            start = re.search(r"^decide\s*\{", source, re.M).start()
            header = source[:start]
            pins = "".join(f"    requires {name} == {value}\n" for name, value in values.items())
            pos = header.index("    ensures")
            header = header[:pos] + pins + header[pos:]
            path.write_text(header + "{ " + alternative + " }\n" + f"example {d['name']}({args}) == {alternative}\n", encoding="utf-8", newline="\n")
            candidate, _ = check(binary, str(path))
            assert all(o["outcome"] == "proved" for o in candidate["definitions"][0]["obligations"]), task
            assert candidate["definitions"][0]["examples"][0]["run"] == "passed", task


def generate(binary):
    out = {"allocation.csv": allocation()}
    for task in KEY:
        name = task["task"]
        path = (HERE / "corpus" / f"{name}.accord").relative_to(ROOT).as_posix()
        report, text = check(binary, path)
        d = validate_category(task, report)
        source = (ROOT / path).read_text(encoding="utf-8")
        validate_witness(binary, task, source, d)
        shared = f"# {name}\n\n{REQUIREMENTS[task['domain']]}\n\nFind requirement violations or missing promises. Give a concrete input and\nrequired result, or an excluded-input witness. Say which claim is affected.\nChoose accept, request_change, or insufficient_information. If an artifact\ncannot reveal a defect, say not_observable. After the task, rate confidence\n0–100. Time limit: eight minutes. No tools, source browsing, or other packets.\n\n"
        for arm, material in [("code", code_only(source, d)), ("specification", spec(d)), ("review", text)]:
            language = "text" if arm == "review" else "accord"
            artifact = f"cards/{arm}/{name}.md"
            attribution = cedar_attribution(artifact) if task["domain"] == "cedar" else ""
            out[artifact] = shared + attribution + f"```{language}\n{material}```\n"
        print(f"validated {name}: {report['standing']}", flush=True)
    for domain, target in [("gate", "conformance/programs/tesota"), ("cedar", "conformance/programs/cedar")]:
        report, text = check(binary, target)
        if domain == "gate":
            names = {d["name"] for d in report["definitions"] if d["file"] == "gate-rule.accord"}
            sections = re.split(r"(?=^function     )", text, flags=re.M)
            text = "".join(s for s in sections if any(re.match(r"function     fn " + re.escape(n) + r"\(", s) for n in names))
            text += f"\nProgram context: the checked folder is {report['standing']}.\n"
            for d in report["definitions"]:
                if d["name"] not in names and d["standing"] != "verified":
                    text += f"Related function {d['name']}: {d['standing']}; " + ", ".join(x["kind"] for x in d["not_proved"] if x["kind"] in ("open_case", "surviving_mutants", "undecided_obligation")) + ".\n"
        intro = """Read only this packet and the glossary below; do not open source or the answer key.
This is a readability check, not E2. No human result is recorded yet. The text
below is generated from real checker output; only cache-use markers are removed.
Verified means checked against the written specification, not approved intent.
An example is one executed call; a row witness establishes existence, not a
universal guarantee. The functions return values and have no external effects.
Text membership is exact equality. Proved/SendBack/NoProgress/Operator are gate
verdicts; Passed/Failed/Vacuous/TimedOut/Cancelled/NotStarted describe proof-run
outcomes. Vacuous means nothing was verified. Cedar residuals depend on unknown
information; Unknown is an authorization answer, not solver uncertainty.
"""
        artifact = f"../stage-7-review/{domain}-packet.md"
        attribution = cedar_attribution(artifact) if domain == "cedar" else ""
        out[artifact] = f"# {domain.title()} readability packet\n\n{intro}\nRecorded command: `accord check {target}`. Display: unwrapped plain terminal\ntext; no graphical terminal viewport was captured. Data: the named conformance\nprogram. Interaction: run the check and read its output.\n\n{attribution}```text\n{text}```\n\n## Questions\n\n{QUESTIONS[domain]}\n\nReturn answers numbered 1–8 and any unclear wording.\n"
    manifest = {name: digest(text.encode("utf-8")) for name, text in sorted(out.items())}
    for path in sorted((HERE / "corpus").glob("*.accord")):
        if path.is_file():
            manifest["corpus/" + path.name] = digest(path.read_bytes())
    for name in ("answer-key.json", "materials.py", "score.py", "test_score.py", "test_materials.py", "README.md"):
        manifest[name] = digest((HERE / name).read_bytes())
    out["material-hashes.json"] = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("accord", type=pathlib.Path)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    expected = generate(args.accord.resolve())
    stale = []
    for name, text in expected.items():
        path = HERE / name
        if args.write:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
        elif not path.exists() or path.read_text(encoding="utf-8") != text:
            stale.append(name)
    assert not stale, "stale generated artifacts: " + ", ".join(stale)
    print(f"PASS: 12 task categories, 36 cards, two readability packets, allocation and hashes ({'written' if args.write else 'current'}).")


if __name__ == "__main__":
    main()
