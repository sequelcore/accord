# Machines before their code: two probes, 2026-11-12

**Status:** two hand-written probes, run on one machine, before any stage 10
code. They inform stage 10's semantics
([#82](https://github.com/sequelcore/accord/issues/82)). Neither one measures
what Accord will generate.

## Questions

1. **opossum.** Can a module that uses only what Accord has (integers, enums,
   `Bool`s, one function per event and observers of the state) replace
   opossum's breaker state and its decisions, with opossum's tests passing as
   they do at baseline?
2. **ProviderRetry.** With its nondeterminism as event parameters, do Z3 and
   cvc5 both prove Tesota's `models/ProviderRetry.tla` inductive once helper
   invariants are added, and refute it without them? Do its planted faults
   reach a violation?

## What runs

[`opossum/run.sh`](opossum/run.sh) clones opossum 10.0.0 at `decbedf`. It
runs [`invariants.cjs`](opossum/invariants.cjs) and
[`stale-success.cjs`](opossum/stale-success.cjs) against the release, then
lint and tests three ways:

- as released;
- with [`machine.ts`](opossum/machine.ts) copied in as `lib/machine.mts` and
  [`adapter.patch`](opossum/adapter.patch) applied;
- with each of three faults planted in the stand-in.

`machine.ts` is a hand-written stand-in for a generated module. It replaces
the flags `STATE`, `PENDING_CLOSE`, `ENABLED`, `WARMING_UP` and `SHUTDOWN`,
the decision in `fail()` (with the error rate compared exactly,
`failures * 100 > threshold * fires`) and the admission check in `call()`.
Emitted events, timers, the cache, the semaphore, the rolling statistics and
fallbacks stay in `circuit.js`.

[`provider-retry/run.sh`](provider-retry/run.sh) writes the model's
obligations with [`gen.py`](provider-retry/gen.py) and runs both solvers on
each, with the options Accord uses: 10 s each, and cvc5 with `--mbqi`. Each
file asserts the negation of a claim, so `unsat` proves it. There is one
obligation for the initial state, and one for each event: the invariants,
the helpers and the event's `requires` imply that each sum it computes fits
`i32` and that the next state satisfies the invariants and helpers. The
script then runs [`explore.py`](provider-retry/explore.py), a breadth-first
search of the same machine's reachable states.

The model's choices become parameters:

- `\E delay \in {1, 60, 121}` becomes `Schedule(delay)`;
- `time' \in {due, 120}` becomes `Advance(next)`;
- `terminal' \in BOOLEAN` becomes `Failure(isTerminal)`, and a parameter of
  the initial state.

The constant `route` and the stuttering `Done` are dropped. The faults
`IgnoreCancel` and `ReplayTool` are arguments to the scripts.

## Identity

- Accord at `197e024`.
- opossum 10.0.0 at `decbedf63d7815049233e544dcb351590ff0c84e`.
- Tesota's model at `4ef9b191`.
- Z3 5.3.3 and cvc5 1.4.2 from `scripts/install-solvers.sh`.
- Node 24.22.1, on Windows 11 Pro, a developer machine whose background load
  wasn't controlled.

## Result

### opossum

| Run | Tests | Failing |
| --- | --- | --- |
| As released | 550 | 4: test 331, a 10 ms rolling-window timing test; tests 547 to 549, a health-check timer that fires after its test ended |
| With the stand-in | 550 | The same 4 |
| Fault: half-open never admits its probe | 550 | 17, and the run stops on an unhandled rejection |
| Fault: `>=` in place of `>` on the threshold | 550 | The same 4 as released |
| Fault: half-open admits every call | 550 | The same 4 as released |

Lint passed in every run. The first two runs were made three times each,
with the same four failures each time.

Against the release:

- `shutdown()` then `close()` closes the breaker, and `fire()` runs the
  action.
- `open()` then `disable()` leaves it open, and `fire()` runs the action.
- Restoring `{ closed: true, halfOpen: true }` gives a closed breaker with
  `pendingClose: true`.
- A call admitted while the breaker is closed that resolves at 400 ms, after
  the 100 ms reset timeout, closes the half-open breaker:
  `open | halfOpen | t=250 halfOpen=true pendingClose=true | close | slow-success`.
  Any `success` while half-open closes it (`lib/circuit.js:347-351`).

### ProviderRetry

| Helper invariants | Not proved (Z3 and cvc5 agree on every file) |
| --- | --- |
| None | `Schedule`: `time = 2147483588` makes `time + 60` overflow `i32`. `StartCall`: `calls = 4` while waiting gives `calls = 5` |
| `calls <= retries + 1`, `phase == waiting implies calls <= retries`, `time <= 120` | `Advance`: `due = 121` while waiting, then `time = 121` |
| The same, and `due <= 120` | None: all eight obligations proved |
| The four, with `IgnoreCancel` | `StartCall`, from `calls = 0` and `stopped` |
| The four, with `ReplayTool` | `StartCall` |

Every counterexample above is a state no run reaches: `Schedule` sets
`due` below 120, `calls` starts at 1, and the search below finds no
violation in the correct model.

| Fault | Reachable states | First violation |
| --- | --- | --- |
| None | 148 | None; every event fires in some reachable state |
| `IgnoreCancel` | 178 | `CancelStopsCalls`, in 4 steps: `Schedule(1)`, `Advance(1)`, `Cancel`, `StartCall` |
| `ReplayTool` | 148 | `ToolsAndRouteStay`, in 3 steps: `Schedule(1)`, `Advance(1)`, `StartCall` |

## What it shows

- **Events and observers suffice for opossum.** The stand-in replaces the
  breaker's state and decisions with no new failure. Since admitting and
  rejecting a call leave the state unchanged, the adapter reads the
  decision from observers (`admits`, `isHalfOpen`).
- **A CommonJS package can't load the module as `.ts`.** In opossum's
  `"type": "commonjs"` package, Node 24 reads a `.ts` file as CommonJS, and
  `export` is a `SyntaxError`. The same file named `.mts` loads through
  `require`.
- **opossum's tests don't check its threshold boundary, or that half-open
  admits one call.** Two of the three planted faults survive the suite, so
  those properties rest on Accord's conformance cases and proofs alone.
- **opossum's breaker doesn't hold three invariants a reader would expect:**
  shutdown is permanent, an open breaker admits no call, and pending close
  implies half-open.
- **The error rate needs `i64`.** `failures * 100` doesn't fit `i32` for
  every count opossum keeps.
- **Parameters cover the model's nondeterminism.** No choice in
  ProviderRetry needed a relation.
- **Helper invariants are needed, and the solvers name what's missing.** Each
  failed obligation came with a state that satisfies the invariants and
  leaves them in one step. Overflow obligations need the invariants as
  assumptions.
- **A failed preservation obligation is not a detected fault.** Both
  planted faults fail preservation, but from states no run reaches. Only the
  search shows that each fault is reachable, and how.

## What it doesn't show

- What `accord build` or Accord's obligation generator will write: both
  stand-ins are hand-written.
- That 148 is the model's state count under TLC; it hasn't been compared.
- Solver times: each file ran once with a 10-second limit, and none timed
  out.
- That the stand-in matches opossum on paths its tests don't reach.
