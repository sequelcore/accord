# Stage 9's proof cost and undecided rate, 2026-11-12

**Status:** one measurement on one machine, before and after stage 9
([#69](https://github.com/sequelcore/accord/issues/69)). It times whole
checks, so a cost per obligation here includes process start-up, mutation
analysis and the open-case search (see **What it doesn't show**).

## Question

Did stage 9 (calls, quantifiers, length and indexing) change the cost of
proving what Accord already proved, or how often an obligation is left
undecided? What do the stage's new ports cost?

## What runs

[`measure.py`](measure.py) copies a `conformance` folder and runs
`accord check <target> --json` on each case and each program in it, with no
ledger, so nothing is reused: one warm-up, then three timed runs, keeping the
median. It counts each obligation's outcome from the JSON. Cases the compiler
rejects, and the `duplicate-name` program, have no obligation and are
skipped.

| Run | Accord | Corpus |
| --- | --- | --- |
| Before | `86b962a`, the merge before stage 9's first code | Its own `conformance`: 84 targets |
| After | `2db3bf6`, with the ports of `verdict-rule` and Cedar's policy-set evaluation, less `some_policy`'s `intent`, which adds no obligation | Its own `conformance`: 104 targets |

Raw results: [`before.json`](before.json), [`after.json`](after.json).

## Identity

- Both built with `cargo build --release --locked` (Rust 1.93.1); Z3 5.3.3
  and cvc5 1.4.2 from `scripts/install-solvers.sh`, 10 seconds per query.
  The base copy's `cargo test --workspace --locked --release` passed (79
  tests) before its run.
- AMD Ryzen 5 5600X, Windows 11 Pro, a developer machine whose background
  load wasn't controlled. The two runs were sequential.

## Result

**What was already proved costs the same.** 77 targets are byte for byte
the same in both corpora. Their 472 obligations have the same outcomes, and
the whole set took 43.1 s before and 41.1 s after: 91 and 87 ms per
obligation. The median ratio of a target's time after to before is 0.94
(from 0.71 to 1.19), within what this machine's noise allows. Tesota's three
earlier rules, checked alone, took 7.26 s before and 7.31 s after; Cedar's
partial decision 1.51 s and 1.36 s.

**Calls made a port cheaper.** The axios-retry port, rewritten with calls in
place of its repeated retry rule, took 2.53 s before and 1.01 s after, with
the same six obligations.

**The new ports cost more per obligation.**

| Target | Obligations | Time | Per obligation |
| --- | --- | --- | --- |
| `programs/tesota`, with `verdict-rule` | 138 (61 before) | 20.0 s (7.3 s before) | 145 ms |
| `verdict-rule` alone, as the difference | 77 | 12.7 s | 165 ms |
| `programs/cedar`, with the policy set | 51 (12 before) | 3.5 s (1.5 s before) | 68 ms |
| Stage 9's 14 cases and 6 fault programs | 439 | 47.7 s | 109 ms |

`verdict-rule`'s obligations quantify over lists, which costs each solver
more than the quantifier-free rules before it.

**Undecided rate.** Before: 0 of 565 obligations. After: 1 of 1,120 is
undecided and 1 is `weak_callee`, both in cases built to give that outcome
(`list-weak-callee.accord`, `call-weak-contract.accord`). No obligation of
either port is undecided.

**One fault left undecided, and dropped.** A planted fault that removed
`run_standing`'s `requires` that the three lists have the same length was
refuted in the body (`tested[k]` out of range), but cvc5 ran out of time or
answered `unknown` on the same index inside the quantified `ensures`, and on
one row's reachability: 5 of 23 obligations undecided. Fail-closed held, but
an outcome that rests on a timeout can change with the machine, so the fault
isn't kept as a conformance program. The indices out of range that the
conformance cases keep (`list-index-fault`, `list-index-spec-fault`) are
refuted outright.

## What it doesn't show

- **Not solving alone.** Each time includes starting two solvers per query,
  replaying models, mutation analysis and the open-case search. How much of
  each total is solving was not separated.
- **Not general.** One machine, one operating system, three runs per target,
  warm file caches. The before and after ratios within 0.8 to 1.0 are noise
  of this setup, not a speed-up.
- **Different corpora.** The after corpus has stage 9's cases and ports; the
  comparison of like with like is the 77 identical targets.

## Rerun

With both builds in release and the solvers' variables set:

```sh
python -I measure.py <accord binary> <conformance folder> 3
```
