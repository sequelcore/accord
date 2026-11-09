# E1. Mechanical open cases

**Status:** registered on 2026-11-08, before the agent arm's run. The
[roadmap](../../../ROADMAP.md) lists it; stage 5
([#27](https://github.com/sequelcore/accord/issues/27)) closes when it has run.

## Question

Does the solver find the ambiguities that asking an agent finds, or more? An
ambiguity here is an **open case**: an input the contract admits for which it
allows two different results ([concepts](../../concepts.md)).

## Material

Ten functions in [`corpus/`](corpus), with their contracts and bodies:

| File | Contract | Known answer |
| --- | --- | --- |
| `a1-positive` | `result implies n >= 0` | open for every `n >= 0` |
| `a2-clamp` | the result lies in `[low, high]` | open whenever `low < high` |
| `a3-larger` | the result is at least `a` and `b` | open for every input |
| `a4-sign` | `1` for positive, `-1` for negative | open only at `n = 0` |
| `a5-proof-outcome` | Tesota's six properties of its rule | open where a started run's regeneration or check fails: `TimedOut` and `Cancelled` are allowed besides `Failed` |
| `a6-next` | the next light differs from the current one | open for every light |
| `c1-larger` | at least `a` and `b`, and one of them | determined |
| `c2-add` | `result == a + b` | determined |
| `c3-weakens` | Tesota's `weakens` equivalence | determined |
| `c4-mutant-finding` | Tesota's three equivalences | determined |

The six `a` files have planted ambiguities, from obvious to subtle (`a4` at a
single point, `a5` in one region of a real rule); the four `c` files have
none. A file's name and this table are never shown to an agent.

**Disclosure.** The solver arm is deterministic, and it was run on the corpus
while the corpus was built, to confirm the known answers: it reported an open
case in each `a` file and none in the `c` files. Its result is therefore
known before registration, and the experiment's information is in the agent
arm.

## Arms

**Solver.** `accord check <file> --json`; an open case is its `openness.open`
witness.

**Agents.** The seven models of the [comprehension
check](../comprehension-check/README.md), through OpenRouter, with
[the prompt](prompt.txt) and one corpus file as the only message: no system
prompt, no tools, a new conversation each time. Each file goes to each model
twice: 140 conversations. The prompt asks for concrete open cases as JSON: the
input's values and two results. A reply that comes back empty is run once
more; if it is empty again, it counts as claiming nothing.

## Scoring

Every claim, an input and two results, is checked by Accord itself, with no
judgment by hand: for each of the two results, a function with the corpus
file's `requires` and `ensures`, a `requires` pinning each parameter to the
claimed input, and the result as its body, must verify; and the two results
must differ. A claim that passes is **valid**; any other claim, including one
that can't be parsed, is **invalid**. The scoring script and its output are
committed with the run.

Per conversation and file:

- **found:** at least one valid claim, for an `a` file;
- **false claims:** the number of invalid claims, for any file.

## Measures and pass mark

- **Agent recall:** the share of the 84 conversations on `a` files that
  found an open case, overall and per file.
- **Agent precision:** valid claims over all claims, overall and on `c` files
  alone, where every claim is invalid by construction.
- **Agent-only finds:** valid claims on a file where the solver reported the
  contract determined. A solver that misses ambiguities agents find would show
  here.

The solver arm **answers the question with "as many or more"** if:

1. there are no agent-only finds; and
2. on at least one `a` file, at least half of the agent conversations didn't
   find an open case.

If (1) fails, the solver missed a real ambiguity, which is reported as a
defect. If (1) holds and (2) doesn't, the solver finds the same as agents, no
more, on this corpus. The pass mark doesn't change after the run.
