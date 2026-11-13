"""Breadth-first exploration of ProviderRetry with the same events and parameters
as gen.py: every reachable state, whether the TLA invariants hold in each, and
the shortest trace to a violation for each planted fault.

Usage: python explore.py [none|IgnoreCancel|ReplayTool]
"""
import sys
from collections import deque

FAULT = sys.argv[1] if len(sys.argv) > 1 else "none"
F = ("phase", "time", "due", "retries", "calls", "stopped", "callsAtStop",
     "terminal", "lastStart", "lastDue", "tools")


def S(**kw):
    return tuple(kw[f] for f in F)


def D(s):
    return dict(zip(F, s))


def steps(s):
    v = D(s)
    if v["phase"] == "failed":
        for delay in (1, 60, 121):
            if not v["terminal"] and not v["stopped"] and v["retries"] < 3 and v["time"] + delay < 120:
                yield f"Schedule({delay})", S(**{**v, "phase": "waiting", "due": v["time"] + delay, "retries": v["retries"] + 1})
            else:
                yield f"Schedule({delay})", S(**{**v, "phase": "ended"})
    if v["phase"] == "waiting" and v["time"] < 120:
        for nxt in {v["due"], 120}:
            if nxt >= v["time"]:
                yield f"Advance({nxt})", S(**{**v, "time": nxt})
    if not v["stopped"] and v["phase"] in ("failed", "waiting", "calling"):
        yield "Cancel", S(**{**v, "stopped": True, "callsAtStop": v["calls"]})
    if v["phase"] == "waiting" and v["time"] >= v["due"] and v["time"] < 120 and (not v["stopped"] or FAULT == "IgnoreCancel"):
        tools = v["tools"] + 1 if FAULT == "ReplayTool" else v["tools"]
        yield "StartCall", S(**{**v, "phase": "calling", "calls": v["calls"] + 1, "lastStart": v["time"], "lastDue": v["due"], "tools": tools})
    if v["phase"] == "waiting" and (v["stopped"] or v["time"] >= 120):
        yield "StopWait", S(**{**v, "phase": "ended"})
    if v["phase"] == "calling":
        for t in (False, True):
            yield f"Failure({str(t).lower()})", S(**{**v, "phase": "failed", "terminal": t})
        yield "Success", S(**{**v, "phase": "ended"})


def violated(s):
    v = D(s)
    checks = {"Bounds": v["retries"] <= 3 and v["calls"] <= 4 and v["lastStart"] < 120,
              "CancelStopsCalls": not v["stopped"] or v["calls"] == v["callsAtStop"],
              "WaitRespected": v["lastStart"] >= v["lastDue"],
              "ToolsStay": v["tools"] == 1}
    return [k for k, ok in checks.items() if not ok]


inits = [S(phase="failed", time=0, due=0, retries=0, calls=1, stopped=False, callsAtStop=0,
           terminal=t, lastStart=0, lastDue=0, tools=1) for t in (False, True)]
parent = {s: None for s in inits}
queue = deque(inits)
fired, first = set(), None
while queue:
    s = queue.popleft()
    if violated(s) and first is None:
        first = s
    for label, t in steps(s):
        fired.add(label.split("(")[0])
        if t not in parent:
            parent[t] = (s, label)
            queue.append(t)

print(f"fault={FAULT}: {len(parent)} distinct reachable states; events that fire: {sorted(fired)}")
if first is None:
    print("  every invariant holds in every reachable state")
else:
    trace, s = [], first
    while parent[s] is not None:
        s, label = parent[s]
        trace.append(label)
    print(f"  violates {violated(first)} after {len(trace)} steps: init -> " + " -> ".join(reversed(trace)))
