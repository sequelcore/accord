"""Writes the inductive obligations of Tesota's ProviderRetry model
(models/ProviderRetry.tla at 4ef9b191) as SMT-LIB files, shaped as Accord
writes obligations: logic ALL, enums as datatypes, i32 values as Int with
their range assumed. Each file asserts the negation of a claim, so unsat
proves it and sat gives a counterexample.

The model becomes a machine whose events are pure functions: each choice the
TLA model leaves open is an event parameter (`Schedule(delay)`,
`Advance(next)`, `Failure(isTerminal)`, and the initial `terminal`), and the
constant `route` and the stuttering `Done` are dropped.

Usage: python gen.py <out folder> <helpers> [fault]
  helpers: none | three | four  (the helper invariants assumed, below)
  fault:   IgnoreCancel | ReplayTool  (the model's planted faults)
"""
import os
import sys

I32 = (-2**31, 2**31 - 1)

FIELDS = [("phase", "Phase"), ("time", "Int"), ("due", "Int"), ("retries", "Int"),
          ("calls", "Int"), ("stopped", "Bool"), ("callsAtStop", "Int"),
          ("terminal", "Bool"), ("lastStart", "Int"), ("lastDue", "Int"), ("tools", "Int")]

# The model's invariants: Bounds, CancelStopsCalls, WaitRespected, ToolsAndRouteStay.
INVARIANTS = ["(<= retries 3)", "(<= calls 4)", "(< lastStart 120)",
              "(=> stopped (= calls callsAtStop))", "(>= lastStart lastDue)", "(= tools 1)"]

HELPERS = {
    "none": [],
    "three": ["(<= calls (+ retries 1))", "(=> (= phase waiting) (<= calls retries))",
              "(<= time 120)"],
    "four": ["(<= calls (+ retries 1))", "(=> (= phase waiting) (<= calls retries))",
             "(<= time 120)", "(<= due 120)"],
}

PRELUDE = ("(set-option :produce-models true)\n(set-logic ALL)\n"
           "(declare-datatypes ((Phase 0)) (((failed) (waiting) (calling) (ended))))\n")


def in_i32(e):
    return f"(and (<= {I32[0]} {e}) (<= {e} {I32[1]}))"


def events(fault):
    """Each event: its parameters, its requires, its next state, and the sums it computes."""
    cur = {n: n for n, _ in FIELDS}
    go = "(and (not terminal) (not stopped) (< retries 3) (< (+ time delay) 120))"
    waiting = dict(cur, phase="waiting", due="(+ time delay)", retries="(+ retries 1)")
    ended = dict(cur, phase="ended")
    start_requires = "(and (= phase waiting) (>= time due) (< time 120)" + \
        ("" if fault == "IgnoreCancel" else " (not stopped)") + ")"
    start = dict(cur, phase="calling", calls="(+ calls 1)", lastStart="time", lastDue="due",
                 tools="(+ tools 1)" if fault == "ReplayTool" else "tools")
    return {
        "Schedule": ([("delay", "Int")],
                     "(and (= phase failed) (or (= delay 1) (= delay 60) (= delay 121)))",
                     {n: f"(ite {go} {waiting[n]} {ended[n]})" for n, _ in FIELDS},
                     ["(+ time delay)", "(+ retries 1)"]),
        "Advance": ([("next", "Int")],
                    "(and (= phase waiting) (< time 120) (or (= next due) (= next 120)) (>= next time))",
                    dict(cur, time="next"), []),
        "Cancel": ([], "(and (not stopped) (or (= phase failed) (= phase waiting) (= phase calling)))",
                   dict(cur, stopped="true", callsAtStop="calls"), []),
        "StartCall": ([], start_requires, start,
                      ["(+ calls 1)"] + (["(+ tools 1)"] if fault == "ReplayTool" else [])),
        "StopWait": ([], "(and (= phase waiting) (or stopped (>= time 120)))", dict(cur, phase="ended"), []),
        "Failure": ([("isTerminal", "Bool")], "(= phase calling)",
                    dict(cur, phase="failed", terminal="isTerminal"), []),
        "Success": ([], "(= phase calling)", dict(cur, phase="ended"), []),
    }


def obligations(helpers, fault):
    j = "(and " + " ".join(INVARIANTS + HELPERS[helpers]) + ")"
    files = {}
    init = {"phase": "failed", "time": "0", "due": "0", "retries": "0", "calls": "1",
            "stopped": "false", "callsAtStop": "0", "terminal": "terminal0",
            "lastStart": "0", "lastDue": "0", "tools": "1"}
    bind = " ".join(f"({n} {init[n]})" for n, _ in FIELDS)
    files["init"] = (PRELUDE + "(declare-const terminal0 Bool)\n"
                     f"(assert (not (let ({bind}) {j})))\n(check-sat)\n(get-model)\n")
    # Preservation, per event: the invariants, the helpers and the event's
    # requires imply that every sum it computes fits i32 and that the next
    # state satisfies the invariants and the helpers.
    for name, (params, requires, nxt, sums) in events(fault).items():
        lines = [PRELUDE.rstrip()]
        lines += [f"(declare-const {n} {s})" for n, s in FIELDS]
        lines += [f"(assert {in_i32(n)})" for n, s in FIELDS if s == "Int"]
        for p, s in params:
            lines.append(f"(declare-const {p} {s})")
            if s == "Int":
                lines.append(f"(assert {in_i32(p)})")
        lines.append(f"(assert {j})")
        lines.append(f"(assert {requires})")
        after = " ".join(f"({n} {nxt[n]})" for n, _ in FIELDS)
        fits = " ".join(in_i32(e) for e in sums) or "true"
        lines.append(f"(assert (not (and {fits} (let ({after}) {j}))))")
        lines.append("(check-sat)\n(get-model)")
        files[f"preserve-{name}"] = "\n".join(lines) + "\n"
    return files


if __name__ == "__main__":
    out, helpers = sys.argv[1], sys.argv[2]
    fault = sys.argv[3] if len(sys.argv) > 3 else "none"
    os.makedirs(out, exist_ok=True)
    for name, text in obligations(helpers, fault).items():
        with open(os.path.join(out, f"{name}.smt2"), "w", newline="\n") as f:
            f.write(text)
