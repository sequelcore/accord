# Checking `proof-outcome-rule`: wall time, 2026-11-08

**Status:** one measurement on one machine. It times the same rule checked
three ways, but the three don't check the same things, so it supports no
claim that one verifier is faster than another (see **What it doesn't show**).

## Question

How long does checking Tesota's `proof-outcome-rule` take end to end, in
Accord and in Tesota's current LemmaScript path, on one developer machine?

## What runs

| Workload | Command | Checks |
| --- | --- | --- |
| Accord | `accord check conformance/cases/proof-outcome.accord` (release build) | 16 obligations (9 `ensures`, coverage, 6 rows reachable), each by Z3 **and** cvc5, every model replayed by the interpreter, and 2 examples run |
| LemmaScript | `lsc check --backend=dafny src/verification/proof-outcome-rule.ts` in Tesota | Regenerates `proof-outcome-rule.dfy.gen` from the TypeScript, then runs Dafny on the `.dfy`: 2 verified items (the function and the lemma holding its six properties), by Z3 alone |
| Dafny alone | `dafny verify src/verification/proof-outcome-rule.dfy` in Tesota | The Dafny step of the row above |

[`measure.py`](measure.py) runs each workload once as a warm-up, then 10 times
in a row, and requires each run to exit 0. Raw times are in
[`result.json`](result.json).

## Identity

- Accord at `39702fe`, built with `cargo build --release` (Rust 1.93.1); Z3
  5.3.3 and cvc5 1.4.2 from `scripts/install-solvers.sh`.
- Tesota at `76f42821`; its `proof-outcome-rule.ts` is the same at `dev`
  `4ef9b191`. `lsc` from its `node_modules`; Dafny 4.11.0, which bundles Z3
  4.12.1 and 4.14.1. Which of the two it ran was not recorded.
- AMD Ryzen 5 5600X, Windows 11 Pro, a developer machine whose background
  load wasn't controlled. Tesota's working tree was unchanged after the runs.

## Result

| Workload | Median | Min to max (10 runs) |
| --- | --- | --- |
| Accord | 0.63 s | 0.55 to 0.65 s |
| LemmaScript (`lsc check`) | 5.65 s | 5.60 to 5.89 s |
| Dafny alone | 0.87 s | 0.85 to 0.93 s |

The ranges don't overlap, so on this machine the order is stable: Accord's
whole check takes about a ninth of `lsc check`'s, and less than Dafny's step
alone.

## What it doesn't show

- **Not the same work.** Accord decides 16 obligations with two solvers and
  replays every model; Dafny proves Tesota's six properties as one lemma with
  one solver. Accord also proves coverage and reachability, which the
  LemmaScript proof doesn't state. A per-obligation or per-property cost isn't
  comparable from these numbers.
- **Not the proof's cost.** Each time includes process start-up: 32
  solver processes for Accord (16 obligations, two solvers each), a JavaScript runtime and regeneration for
  `lsc`, and the .NET runtime for Dafny. How much of each total is solving was
  not separated.
- **Not general.** One small rule with no quantifiers, one machine, one
  operating system, warm file caches. Larger rules, other platforms or a cold
  start may order the three differently.
- **Different solver versions.** Accord's Z3 is 5.3.3; Dafny's is 4.12.1 or
  4.14.1.

## Rerun

With Accord built in release, the solvers' variables set
(`scripts/install-solvers.sh`), Dafny on the `PATH` and Tesota's dependencies
installed:

```sh
python -I measure.py <accord checkout> <tesota checkout> 10
```
