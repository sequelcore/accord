# Semver port's obligations before its code: solver answers, 2026-11-13

**Status:** hand-written obligations of the node-semver port, shaped as
Accord's compiler writes them, run on one machine before any stage 11 code.
They inform stage 11's contract for precedence
([#83](https://github.com/sequelcore/accord/issues/83)). They are not the
obligations Accord will generate: there is no compiler for records, sums or
`match` yet.

## Question

Stage 11's decisions say the port follows SemVer §11 and that the comparator's
contract states precedence in two forms, existential and universal, as
separate `ensures`. Are the port's obligations then proved by both pinned
solvers, with the slowest run under 5 s, and are three planted faults refuted
by both?

## Success rule

Stated before the results:

- Every proof obligation is `unsat` in Z3 and in cvc5, and its slowest of 5
  runs takes under 5 s in each.
- Each planted fault is `sat` in both solvers on at least one clause.
- The open-case search of each comparator is `unsat` in both: the contract
  determines the result.

## What runs

[`gen.py`](gen.py) writes [`cases/`](cases), 58 files. [`run.sh`](run.sh) runs
each one five times with `z3 -smt2 -t:10000` and `cvc5 --lang smt2 --mbqi
--tlimit-per=10000`, the options Accord uses, and prints the answers and the
minimum, median and maximum time in milliseconds. [`results.txt`](results.txt)
is its output. Each file asserts the negation of a claim: `unsat` proves it,
`sat` refutes it. A fault's case asserts the faulty body in place of the
port's, and one clause of its contract; `unsat` there means that clause
doesn't notice the fault.

The model follows the owner decisions. `PreId` is an SMT datatype
(`Numeric(digits)`, `Alpha(text)`); a version has `major`, `minor`, `patch`
(`i64`) and `pre`, `build` (`Seq PreId`); and the adapter guarantees that
numeric digits are canonical, which every `requires` assumes. Text order is
`str.<`, code point order. As Accord generates them, each integer is within
its range, each list holds at most 4,294,967,295 elements, a result is an
`i32`, and a callee is known only through its `requires` and `ensures`, as
predicates. Each comparator returns -1, 0 or 1. "Greater" is stated as
"neither less nor equal", since cvc5 proves no order law of `str.<`.

| Function | Contract |
| --- | --- |
| `cmp_id(x, y)` | `sign` (the result is in -1..1), `lt`, `eq`, `gt` |
| `cmp_list(a, b, k)` | the identifiers from `k` on, recursive on `k + 1` with `decreases length(a) - k`. Separate `ensures`: `sign`, `eq`, and for less and greater a universal form (`-u`: at the first index where the lists differ or end) and an existential form (`-e`) |
| `compare_main` | `sign`, `eq`, `lt`, `gt` |
| `compare` | the same clauses as `cmp_list`, for precedence: build ignored, a version without a pre-release above one with |
| `compare_build` | precedence, then the build identifiers as a tie-break; `eq` means the versions are identical |

The obligations are:

- one per `ensures` clause;
- for each call, that its callee's `requires` holds where it runs;
- for `cmp_list`, that `a[k]` and `b[k]` are in range, that `length(a) - k`
  decreases and stays non-negative at the recursive call, and that `k + 1` and
  `length(a) - k` fit `i64`;
- the open-case search of each function: two different results that both meet
  every `ensures`.

The faults, each against the body of `cmp_list` or `cmp_id` and every clause:

- `prefix`: the shorter list is greater.
- `skip`: a numeric tie returns 0 instead of going on to the next identifier.
- `numeric`: a numeric identifier is above an alphanumeric one.

## Identity

Accord at `ac1d4ca`, the commit the cases were written at, on
`docs/stage-11-decisions`. Z3 5.3.3 and cvc5 1.4.2
from `scripts/install-solvers.sh`. Windows 11 Pro, a developer machine whose
background load wasn't controlled.

## Result

Times are the median of 5 runs in milliseconds; a time over 10,000 is the
solver's limit. The minima and maxima are in `results.txt`.

| Obligation | Z3 | cvc5 |
| --- | --- | --- |
| `cmp_id`: every clause, open search | unsat, ≤ 120 | unsat, ≤ 120 |
| `compare_main`: every clause, open search | unsat, ≤ 120 | unsat, ≤ 120 |
| `cmp_list`: calls' `requires`, bounds, `decreases`, overflow, `sign`, `eq` | unsat, ≤ 230 | unsat, ≤ 150 |
| `cmp_list` `lt-u` | unsat, 3,798 (max 4,005) | unsat, 321 |
| `cmp_list` `gt-u` | **unknown**, 10,219 | unsat, 261 |
| `cmp_list` `lt-e` | **unknown**, 12,922 | unsat, 137 |
| `cmp_list` `gt-e` | **unknown**, 11,542 | unsat, 147 |
| `cmp_list` open search | unsat, 2,931 | unsat, 120 |
| `compare` `sign`, `lt-e`, `gt-e`, `gt-u`, calls | unsat, ≤ 340 | unsat, ≤ 130 |
| `compare` `lt-u` | unsat, 1,568 | unsat, 103 |
| `compare` `eq` | **unknown**, 10,126 | unsat, 114 |
| `compare` open search | unsat, 8,586 (max 9,097) | unsat, 127 |
| `compare_build` `sign`, `lt-*`, `gt-*`, calls | unsat, ≤ 550 | unsat, ≤ 110 |
| `compare_build` `eq` | **unknown**, 10,095 | unsat, 101 |
| `compare_build` open search | **unknown**, 10,458 | unsat, 169 |

Faults (`sat` means the clause catches it):

| Fault | Z3 | cvc5 |
| --- | --- | --- |
| `numeric` on `cmp_id` `lt` and `gt` | sat | sat |
| `prefix` on `cmp_list` `lt-u`, `lt-e`, `gt-u`, `gt-e` | **unknown** (10 s) | sat |
| `prefix` on `cmp_list` `eq` and `sign` | unsat | unsat |
| `skip` on `cmp_list` `eq` | **unknown** | **unknown** |
| `skip` on the other clauses | unsat | unsat |

Alternative shapes, one run each. They are not in `cases/`; `SHAPE=` and
`FORMS=` in `gen.py` write them:

| Shape | Result |
| --- | --- |
| `SHAPE=negated`: the universal form as "not equal and not greater" | Z3 `unknown` on all four precedence clauses and on the `prefix` fault; no better |
| `SHAPE=inline`: `first` written out in each form | the same |
| `FORMS=e`: the existential form only | Z3 proves `lt-e` (880) and `gt-e` (1,903) and decides the open search (275); it still doesn't refute `prefix` |
| `FORMS=u`: the universal form only | Z3 proves `lt-u` (1,803), not `gt-u`; the open search is `unknown` in both solvers |
| Z3 `smt.arith.solver=2`, which Accord doesn't set | `lt-u` and `gt-u` unsat in about 1 s and the open search in 1 s; the existential forms and the list faults are still `unknown` |

## What it shows

- **The rule is not met.** Z3 doesn't prove `gt-u`, `lt-e` and `gt-e` of the
  recursive comparator, nor `eq` of `compare` and `compare_build`, nor the
  open search of `compare_build`, within 10 s. cvc5 proves every obligation in
  under 0.4 s.
- **Z3 doesn't refute the `prefix` fault, and neither solver refutes
  `skip`.** `numeric` is refuted by both, since it concerns one identifier. A
  list fault needs a model of a list under a quantified contract: cvc5 finds
  one for `prefix`, Z3 finds none in 10 s, and for `skip` the only clause that
  can notice it, `eq`, is `unknown` in both.
- **The two forms cost differently in Z3.** Stated alone, the existential
  form is provable and decides the open search, and the universal form is
  provable only with another arithmetic solver and leaves the open search
  `unknown`. Stated together, as the decision has it, the existential clauses
  of a callee's contract stay unproved for the recursive comparator.
- **Nothing tried made Z3 refute a list fault.** Three shapes of the contract
  and one solver option left `prefix` and `skip` `unknown`.

## What it doesn't show

- It doesn't measure the obligations Accord will generate: sums, records and
  `match` don't exist yet, and the model's names and clause layout are mine.
- It doesn't run Accord's solver wrapper, so a model that the interpreter
  can't replay isn't tested, and a fault is judged by `sat` alone.
- Five runs on one loaded machine give a range, not a distribution.
- The alternatives were one run each, and few were tried; a shape that meets
  the rule may exist.
- It doesn't check that the contract says what SemVer says, only that the
  solvers decide it.

# Second round: Z3 options, other contract shapes, replayed counterexamples

**Status:** run on 2026-11-13 after the first round, on the same model. It asks
what would make Z3 prove the obligations and what could refute the planted
faults. It measures; it chooses nothing.

## Questions

1. With `smt.arith.solver=2` and at most two other documented Z3 options, is
   every proof obligation `unsat` in Z3 under 5 s, with cvc5 unchanged? Does
   the option change Accord's conformance suite?
2. Which contract shapes not yet tried meet the success rule, with and without
   the option?
3. For each planted fault, is there a solver whose `sat` model, decoded and run
   through a reference evaluator, shows the faulty function breaking the
   clause?

The success rule above stays as written. One line is added and kept apart:

- **If refutation by a replayed counterexample were allowed:** every proof
  obligation and open search as in the success rule, and each planted fault
  `sat` in at least one solver on at least one clause, with that model
  replaying in [`replay.py`](replay.py).

Today a function's obligation is refuted only when both solvers find a
counterexample (`combine` in `crates/accord/src/solver/combine.rs`), and a
solver's `unknown` next to the other's `sat` is undecided. The added line is
the rule that would let one solver's replayed model refute.

## Method

**Shapes.** `gen.py` takes `CONTRACT=` and `EQ=` and writes each shape as 48 to
76 cases, named as in the first round. [`results-round2/`](results-round2)
holds the output. The generated folders aren't committed:
`CONTRACT=e EQ=versions python gen.py variants/e-versions` writes one, and
`CASES=variants/e-versions ... run.sh` runs it.

| Shape | Generator | What the list comparator and the two version comparators say |
| --- | --- | --- |
| `both` | default | the first round's contract: `eq`, then universal and existential for less and greater |
| `versions` | `EQ=versions` | as `both`, with a version's `eq` saying its pre-release and build identifiers are equal element by element, not as sequences |
| `e` | `CONTRACT=e` | the existential form only, in every comparator |
| `e-versions` | `CONTRACT=e EQ=versions` | both changes |
| `e-prefix-versions` | `CONTRACT=e-prefix EQ=versions` | `e-versions`, and for lists "one a strict prefix of the other gives -1 or 1" |
| `pin` | `CONTRACT=pin` | a helper `ensures` `pin` (a first differing index exists), then the universal form for less and greater; no existential form |
| `cases` | `CONTRACT=cases` | one clause per case of the inputs: equal gives 0; one a strict prefix of the other gives -1 or 1; at the first differing index, the identifier order gives -1 or 1; each comparator states its own cases |
| `cases-pin` | `CONTRACT=cases-pin` | `cases` and `pin` |

Each shape says what §11 says, by these arguments:

- The first index from `k` where the lists differ or one ends exists and is
  unique. So the existential form ("at that index, the list is less") implies
  the universal form ("at every such index"), and the universal form alone is
  vacuous if no such index exists. `e` and `e-versions` are the existential form;
  `pin` is the universal form plus the clause that the index exists. Each,
  with `eq` and `sign`, fixes the result.
- Equal lists have the same length and the same elements, so `versions` says
  what `both` says.
- `e-prefix-versions` adds a clause the existential form already implies (the
  first index is the shorter list's length).
- The five antecedents of `cases` cover the inputs only if the first
  differing index exists, which is why `cases-pin` adds `pin`.

**Runs.** [`run.sh`](run.sh) takes `CASES=` (a folder of cases), `Z3_OPTS=`
(more Z3 options after Accord's) and `ONLY=z3|cvc5`. Each case runs 5 times, as
before. Each Z3 run is one `run.sh` process, and three to six ran at once on
the 12-thread machine; the first round ran alone. That makes the times here
about twice the first round's for the same Z3 (`both`, `lt-u`: 3.8 s, now up
to 8.0 s), so a time near 5 s is not a reliable line. One run of the best
candidate alone is listed separately. [`summary.py`](summary.py) applies the
success rule to two result files; [`results-round2/summary.txt`](results-round2/summary.txt)
holds its output for every pair.

**Z3 options.** Beside `smt.arith.solver=2`, I tried about 35 single settings
(a few were rejected as unknown parameters), one run each with a 6 to 8 s limit, on the two obligations that stay unproved in `both`
(`cmp_list` `lt-e` and `compare` `eq`), each together with `smt.arith.solver=2`:
`smt.mbqi=false`, `smt.ematching=false`, `smt.relevancy=0` and `1`,
`smt.qi.eager_threshold`, `smt.qi.cost`, `smt.mbqi.max_iterations`,
`smt.mbqi.max_cexs`, `smt.pull_nested_quantifiers`, `smt.macro_finder`,
`smt.restart_strategy`, `smt.random_seed`, `smt.phase_selection`,
`smt.string_solver=seq`, `smt.arith.nl=false`, `smt.seq.*` (three),
`smt.auto_config=false`, `smt.induction`, `smt.qi.persist_instances`,
`smt.q.lift_ite=2`, `smt.q.lite`, `smt.distribute_forall`,
`smt.theory_case_split`, `smt.theory_aware_branching`,
`smt.qi.max_multi_patterns`, `smt.mbqi.force_template`, `smt.recfun_finder`,
`smt.candidate_models`, `tactic.default_tactic=smt`. None proved either.
`sat.smt=true` isn't supported for these scripts. Then, on `e-versions`, I tried 25
settings on the two obligations that stay unproved there (`compare_build`
`gt-e` and `lt-e`), and `smt.relevancy=1` and
`smt.arith.eager_eq_axioms=false` proved them fastest. Their
justification is that screen, not a mechanism I can state. On
`e-prefix-versions`, 15 settings on `lt-e` and `gt-e` of `cmp_list` no setting proved
both. The screen is
[`results-round2/screen-single-run.txt`](results-round2/screen-single-run.txt):
six option sets on five shapes, one run per case. In this Z3, `smt.arith.solver`
defaults to 6; 2 is the simplex solver.

**Replay.** [`replay.py`](replay.py) runs each fault case, with the solver's
`get-value` for the inputs appended, with Z3, Z3 with `smt.arith.solver=2`,
Z3 with `smt.mbqi=false smt.ematching=false` (no quantifier instantiation: its
`unsat` stays sound, its `sat` is only a candidate, and a model that doesn't
replay is discarded) and cvc5. It decodes the model, runs the faulty function
and the contract's clause on it, and calls the model a counterexample that
replays when the inputs meet `requires` and the clause is false for the
result. The evaluator is written from this document's model. It isn't Accord's
interpreter. Each model came from one run.

**Conformance.** `ACCORD_Z3` names a `.cmd` file, which `locate` accepts (it
checks `is_file`) and `std::process::Command` runs. The wrapper answers
`--version` with its own line, so the evidence ledger, keyed by the tools'
versions, isn't shared between runs, and passes every other argument on with
the options appended:

```
@echo off
if "%~1"=="--version" (echo Z3 version 5.3.3 wrapper a2-r2& exit /b 0)
"<path>\z3.exe" %* smt.arith.solver=2
```

`(get-option :smt.arith.solver)` through it returns 2. Four runs of `cargo test
--workspace --locked` followed, one per option set, the plain wrapper first.

## Second-round result

### Question 1: Z3 options on the owner's contract

Proof obligations met in Z3, of 37, and open searches met, of 5, median of 5
runs; cvc5 meets 37 and 5 in every row.

| Z3 options | Proofs | Open searches | Not `unsat` under 5 s |
| --- | --- | --- | --- |
| none | 31 | 3 | `gt-u`, `lt-e`, `gt-e` of `cmp_list` and `eq` of `compare` and `compare_build`, `unknown`; `lt-u` 8.0 s under this load; the open searches of `compare` and `compare_build` `unknown` |
| `smt.arith.solver=2` | 33 | 4 | `lt-e`, `gt-e` of `cmp_list`, `eq` of `compare` and `compare_build`; `compare_build` open search `unknown` |
| `smt.arith.solver=2 smt.relevancy=1` | 32 | 4 | the same, and `lt-u` 5.1 s |
| `smt.arith.solver=2 smt.arith.eager_eq_axioms=false` | 31 | 4 | one run per case (the screen) |
| `smt.arith.solver=2 smt.relevancy=1 smt.arith.eager_eq_axioms=false` | 31 | 4 | one run per case (the screen) |

No. With the contract as the owner decided it, no combination of
`smt.arith.solver=2` and two options proves all 37. The existential clauses
of the recursive comparator and the two `eq` clauses stay `unknown`. With
`EQ=versions` (`versions`), `smt.arith.solver=2` leaves 3 (`lt-e`, `gt-e`,
`compare_build` `gt-e`).

**Conformance.** The four wrappers give the same 97 passed, 1 ignored,
0 failed, and per-test outcomes that are identical
([`conformance.txt`](results-round2/conformance.txt)):

| Z3 options | `cargo test --workspace --locked` |
| --- | --- |
| none | 229 s |
| `smt.arith.solver=2` | 226 s |
| `smt.relevancy=1` | 226 s |
| `smt.arith.solver=2 smt.relevancy=1` | 224 s |

One run each, on the suite's own obligations, which are not semver-sized: it
shows that no test outcome changes, not that the hard obligations don't slow.

### Question 2: shapes

Z3 columns are the median of 5 runs. Each cell is proofs met, of the shape's
total, and open searches met, of 5. cvc5 meets every proof in every shape; its
open searches are 5 of 5 except in `cases` (2 of 5: the three searches whose
cases need `pin` are `unknown`).

| Shape (proofs) | Z3 none | Z3 `smt.arith.solver=2` | Z3 with another option |
| --- | --- | --- | --- |
| `both` (37) | 31, 3 | 33, 4 | 32, 4 (`a2` and `relevancy=1`) |
| `versions` (37) | 34, 5 | 34, 4 | 31, 5 (`relevancy=1`); 34, 4 (`a2` and `relevancy=1`) |
| `e` (31) | 28, 5 | 26, 5 | 28, 5 (`relevancy=1`) |
| `e-versions` (31) | 29, 5 | 27, 5 | **31, 5 (`relevancy=1`)**; 30, 5 (`a2`, `relevancy=1`, `eager_eq_axioms=false`) |
| `e-prefix-versions` (33) | 30, 5 | | 31, 5 (`relevancy=1`) |
| `pin` (35) | 32, 3 | 31, 4 | |
| `cases` (49) | 41, 2 | 44, 2 | |
| `cases-pin` (53) | 46, 4 | 48, 4 | |

`e-versions` with `smt.relevancy=1` meets the proof and open-search part of
the rule in both solvers. Z3's slowest `unsat` is 2.1 s in the runs that shared
the machine and 2.0 s in a run alone (`cmp_list` `lt-e`; `compare_build` `lt-e`
1.8 s); cvc5's is 1.1 s. It isn't robust: with `smt.random_seed=1` and 3
(a single run per case, three runs at once), `compare_build` `gt-e` takes 7.4 s
and `cmp_list` `lt-e` is `unknown`; seed 2 passes (slowest 4.6 s).

The unmet proofs elsewhere are, in `e-prefix-versions` with `smt.relevancy=1`,
`lt-e` and `gt-e` of `cmp_list`; in `pin`, `lt-u` (and, with the option,
`gt-u`) and the `eq` clauses; in `cases`, `diff-lt` and `diff-gt` of `cmp_list`
(not with the option) and, in `compare_build`, `sign` and the `diff` cases; in
`cases-pin`, the existence of the first index (`pin`), which no option made Z3
prove, and the same `compare_build` clauses.

Faults. The cell is the clauses that are `sat`; Z3 is `sat` on `numeric` only,
whatever the shape or option.

| Shape | `numeric` | `prefix` (cvc5) | `skip` (cvc5) |
| --- | --- | --- | --- |
| `both`, `versions` | both: `lt`, `gt` | `lt-u`, `lt-e`, `gt-u`, `gt-e` | none |
| `e`, `e-versions` | both: `lt`, `gt` | `lt-e`, `gt-e` | none |
| `pin` | both | `lt-u`, `gt-u` | none |
| `e-prefix-versions` | both | `lt-e`, `gt-e`, `short`, `long` | `short`, `long` |
| `cases`, `cases-pin` | both | `short`, `long` | `short`, `long`, `diff-lt`, `diff-gt` |

**The success rule is met by no shape and no option.** The proof and
open-search part is met only by `e-versions` with `smt.relevancy=1`. The fault
part needs Z3 to say `sat` on a list fault, and it never did with its options as
Accord runs it, so it fails in every row. The line for replayed
counterexamples is met by none either: `e-versions` with `smt.relevancy=1` has
no clause that notices `skip` (the only one is `eq`, which both solvers leave
`unknown`), and the shapes that do notice it, `e-prefix-versions`, `cases` and
`cases-pin`, are not provable by Z3 (31 of 33, 44 of 49, 48 of 53 at best) or,
for `cases`, decided by cvc5.

In the plain-Z3 runs of seven shapes, no case had one solver `sat` and the
other `unsat`.

### Question 3: replayed counterexamples

All 64 `sat` models in the replay runs replayed. None was spurious.

| Fault | Solvers whose model replays, `both` shape | In the other shapes |
| --- | --- | --- |
| `numeric` | Z3 and cvc5, on `lt` and `gt` | the same |
| `prefix` | cvc5 only, on `lt-u`, `lt-e`, `gt-u`, `gt-e`. Z3 has none, with or without `smt.arith.solver=2` | `e-prefix-versions`, `cases`: cvc5 on `short` and `long` |
| `skip` | none | `e-prefix-versions`: cvc5 on `short` and `long`; `cases`: cvc5 on `short`, `long`, `diff-lt`, `diff-gt` |

A small search over lists of up to two identifiers ([`replay.py --search`](replay.py))
finds an input that violates each of those clauses, and finds none that
violates `sign`, `eq` (for `prefix`), `same` or `pin`. So the planted faults
aren't equivalent mutants of the clauses that are `unknown`; for `skip`, `eq` is
violated by `[0]` against `[0, 0]`.

Z3 with `smt.mbqi=false smt.ematching=false` found models that replay for
`prefix` and `skip` on `short` and `long` in `cases` and for `prefix` on `lt-u`
in `pin`. In `cases` that makes both solvers `sat` on each list fault, as the
original rule asks, but `cases` fails the proofs.

Under the replayed-counterexample rule, `numeric` and `prefix` are refuted in
the owner's contract and `skip` is not: no solver's `sat` exists to replay.
`skip` is refuted by a replayed counterexample only in a contract with the
case clauses (`short`, `long`), which Z3 can't prove.

## What the second round shows

- **Z3's `smt.arith.solver=2` isn't enough, and the second option is luck.** It
  proves the universal clauses but leaves the existential ones. The option that
  gets a shape through, `smt.relevancy=1`, does it for one shape and for two
  of four seeds.
- **Stating the contract differently helps more than an option.** Element-wise
  equality of a version's identifiers (`versions`) removes the `eq`
  `unknown`s with no option: 31 to 34 proofs. The existential form alone is
  provable by cvc5 and, with `smt.relevancy=1`, by Z3, and it implies the
  universal form, which then says nothing more and costs Z3.
- **Whether a fault is refuted depends on the clauses, then on the solver.**
  `skip` is noticed in the owner's contract by `eq` alone, and no solver
  refutes that. A clause for one list a prefix of the other (`short`, `long`)
  notices both list faults, and cvc5 refutes them with models that replay;
  Z3 doesn't.
- **A replayed counterexample from cvc5 alone would refute `prefix` today's
  rule leaves undecided.** `prefix` in the owner's contract is `sat` in cvc5
  and `unknown` in Z3 on all four clauses.

## What the second round doesn't show

- The shapes are mine, and few. A shape that Z3 proves and cvc5 refutes with
  may exist.
- The options were found by screening, and Z3's answers move with the seed:
  the line between `unsat` and `unknown` isn't a property of the contract. A
  clause added elsewhere can flip it.
- The times come from three to six runs at once on one machine, five runs each.
- The replay is my evaluator of the model, not Accord's interpreter, and the
  obligations are the hand-written ones, as in the first round. Whether
  Accord's interpreter replays these models isn't tested.
- The conformance suite is one run per option set. It has no
  semver-sized obligation.
- The `cases-pin` faults weren't replayed: `cases-pin` adds only `pin` to
  `cases`, and `pin` notices neither list fault (cvc5 says `unsat`; Z3 says
  `unsat` in `pin` and `unknown` in `cases-pin`).

# Third round: triggers, spec functions, stability, bugs caught by running the contract

**Status:** run on 2026-11-13 after the second round, on the same model. The
owner's decision for this round: the base shape is `e-versions`
(`CONTRACT=e EQ=versions`), the existential form with element-wise equality
of a version's identifiers; the two-form contract is no longer the target.
It measures four ideas and chooses nothing.

## Questions

A. Do explicit `:pattern`s on every quantifier of `e-versions` make Z3 prove
what it didn't, and do they carry the proof (pure e-matching)?
B. Does a recursive spec function in place of `exists` and `forall`, as
`define-fun-rec` (B1) or as an uninterpreted function with a fuel-limited
defining axiom (B2), meet the success rule?
C. How stable are the answers of the base shape, the best A shape and the best
B shape under seeds, assertion shuffles and renamings?
D. Which planted faults does running the contract on small inputs catch?

The success rule stays as in the first round.

## Method

**Shapes.** `gen.py` has two new switches. `TRIGGERS=1|2|3` adds a `:pattern`
to every quantifier over a list index of the shape (`add_triggers`):

| Design | Pattern |
| --- | --- |
| 1 | the element of the first list the quantifier indexes, `(seq.nth a i)`, as Dafny and Verus pick it |
| 2 | each list's element as an alternative pattern |
| 3 | the elements of both lists as one multi-pattern |

A quantifier that names its index only through `first`, `lt_at` or `gt_at`
takes the lists from that call. No pattern builds a new index term, so none
can loop.

`SPEC=b1|b2f1|b2f2` (only with `CONTRACT=e EQ=versions`) removes `first`,
`eq_from` and `canon_list` and states the contract through two recursive spec
functions:

```
fd(a, b, k)          = k if k >= len a or k >= len b
                       fd(a, b, k + 1) if a[k] = b[k]
                       k otherwise              ; first index from k where they differ or end
canon_from(a, k)     = k >= len a or (canon_id(a[k]) and canon_from(a, k + 1))
lt-e: result = -1 => lt_at(a, b, fd(a, b, k))     gt-e likewise
eq:   result = 0  => fd(a, b, k) = len a and fd(a, b, k) = len b
```

A version's `eq` uses `fd` on the pre-release and build identifiers. A
comparator `requires` `canon_from` from the index it starts at (0 for the
version comparators), so the recursive call's `requires` follows from the
contract's by one unfolding. The obligation of a recursive call uses one
unfolding of `fd` and the callee's contract at the new index.

- B1: `define-fun-rec`.
- B2: an uninterpreted `fd_f(fuel, a, b, k)` with the axiom
  `fd_f(S f, a, b, k) = body[fd_f(f, a, b, k + 1)]`, pattern `fd_f(S f, a, b,
  k)`, and the axiom `fd_f(f, a, b, k) = fd_f(F0, a, b, k)` (the value doesn't
  depend on the fuel), pattern `fd_f(f, a, b, k)`. The contract calls it with
  fuel `S F0` (`b2f1`) or `S (S F0)` (`b2f2`). `canon_from` is built the same
  way.

Generated folders aren't committed. The generated variants for the runs below:

```
CONTRACT=e EQ=versions python gen.py <dir>                     # base
CONTRACT=e EQ=versions TRIGGERS=1 python gen.py <dir>          # t1 (2, 3 likewise)
CONTRACT=e EQ=versions SPEC=b1 python gen.py <dir>             # b1 (b2f1, b2f2 likewise)
```

**Runs.** [`measure.py`](measure.py) runs a folder with the options of
`run.sh`, up to 4 solver processes at once (5 runs per case and solver; its
output has `run.sh`'s layout, so `summary.py` reads it). A solver that exits
without an answer is recorded as `crash<exit code>`. Times from these runs
were taken with four processes at once and, for A and B, with
`enumerate_inputs.py` running beside them in part, so they are about as loose
as the second round's. The cases near 5 s were repeated alone
(`results-round3/alone-*.txt`, `--jobs 1`).

Configurations: Z3 as Accord runs it (`-smt2 -t:10000`), plus `smt.relevancy=1`
(the second round's finding) and `smt.mbqi=false` (e-matching only); cvc5 as
Accord runs it (`--mbqi`) and without `--mbqi`. Without `--mbqi` cvc5 isn't
pure e-matching: its other quantifier modules stay on.

**Stability.** [`stability.py`](stability.py) runs each proof obligation and
open search (36 cases) in 21 variants per solver: as written; Z3 with
`smt.random_seed=N sat.random_seed=N` and cvc5 with `--seed=N
--sat-random-seed=N`, N = 1 to 10; the assertions shuffled (5 seeds); the
symbols renamed (5 seeds). [`mutate.py`](mutate.py) writes the two mutators
as Mariposa does them (Mariposa itself, github.com/secure-foundations/mariposa,
wasn't tried). A case is **stable** if all 21 runs are `unsat`, **unsolvable**
if none is, **unstable** otherwise; a stable case is **slow** if its slowest
run reaches 5 s. Raw rows: `results-round3/C-*.csv`.

**Inputs.** [`enumerate_inputs.py`](enumerate_inputs.py) runs each comparator,
correct and with each fault, on every input of a small domain and checks the
`e-versions` clauses on the result. The evaluator is written from this
document's model, like `replay.py` (which it imports); it isn't Accord's
interpreter.

## Part A: triggers

Cells are proofs met of 31 and open searches met of 5 (all unsat, the proofs
under 5 s in every run); medians of 5 runs; `results-round3/A-*.txt`.

| Shape | Z3 plain | Z3 `relevancy=1` | Z3 `mbqi=false` | cvc5 `--mbqi` | cvc5 no `--mbqi` |
| --- | --- | --- | --- | --- | --- |
| no patterns (base) | 28, 5 | 31, 5 | 29, 5 | 31, 5 | 31, 5 |
| `TRIGGERS=1` | 21, 2 | 21, 2 | 21, 2 | 31, 5 | 31, 5 |
| `TRIGGERS=2` | 21, 2 | 21, 2 | 21, 2 | 31, 5 | 31, 5 |
| `TRIGGERS=3` | 21, 2 | 21, 2 | 21, 2 | 31, 5 | 31, 5 |

With any design Z3 answers `unknown` (10 s) on the same 13 cases in every
configuration: for `cmp_list`, the call's `requires` of `cmp_id`, `eq`,
`lt-e`, `gt-e` and the open search; for `compare` and `compare_build`, `eq`,
`lt-e`, `gt-e` and the open search. Base Z3 plain: `compare_build` `gt-e`
`unknown`; `cmp_list` `gt-e` 5.5 s and `compare_build` `lt-e` 7.2 s
(6.3 s alone). cvc5 proves all 36 with and without patterns and with and
without `--mbqi`, slowest 111 ms.

- Patterns don't help Z3 here; they remove proofs it had, even with
  `smt.mbqi=false`, where the base shape is at 29 of 31.
- The base shape's proofs don't depend on patterns: Z3 with e-matching only,
  and cvc5 without `--mbqi`, prove nearly as much as with MBQI.
- I tried three designs, the allowed maximum; others may behave differently.
  The patterns of design 1 are the ones Dafny and Verus would pick for these
  bodies by my reading; I didn't check against either tool.

Faults, each cell the clauses on which the solver says `sat`:

| Shape | `numeric` | `prefix` | `skip` |
| --- | --- | --- | --- |
| base, any option | Z3 and cvc5: `lt`, `gt` | cvc5 with `--mbqi`: `lt-e`, `gt-e`; Z3: none; cvc5 without `--mbqi`: none | none |
| `TRIGGERS=1` | the same | Z3 plain and `mbqi=false`: `lt-e`; cvc5 with `--mbqi`: `lt-e`, `gt-e` | none |
| `TRIGGERS=2`, `3` | the same | cvc5 with `--mbqi` only | none |

The `lt-e` models of Z3 and cvc5 for `TRIGGERS=1` replay
(`results-round3/A-replay-t1.txt`). Z3's `sat` appeared in 2 of the 3
configurations of that shape that ran, one run each; I didn't test it for
repeatability.

## Part B: spec functions

Cells are proofs met of 31 and open searches met of 5, medians of 5 runs;
`results-round3/B-*.txt`.

| Shape | Z3 plain | Z3 `relevancy=1` | Z3 `mbqi=false` | cvc5 `--mbqi` | cvc5 no `--mbqi` |
| --- | --- | --- | --- | --- | --- |
| B1 `define-fun-rec` | 30, 5 | 30, 5 | 30, 5 | 31, 5 | 31, 5 |
| B2 fuel 1 | 31, 5 | 31, 5 | 31, 5 | 31, 5 | 31, 5 |
| B2 fuel 2 | 31, 5 | 31, 5 | not run | 31, 5 | 31, 5 |

B1's Z3 miss is the call's `requires` of `cmp_id` (`unknown`): the identifiers
are canonical from `k` on, so `a[k]` is canonical needs one unfolding of
`canon_from`, which Z3's `define-fun-rec` didn't do. The slowest proof of any B
cell is 1.2 s (cvc5, B2 fuel 2, `--mbqi`); for B2 fuel 1 it is 0.2 s in both
solvers.

Faults:

| Shape | Z3 | cvc5 |
| --- | --- | --- |
| B1 | `numeric`: `lt`, `gt` `sat`; `skip`: `eq` `sat` (plain only, not with `relevancy=1`); `prefix`: `unknown` on `lt-e`, `gt-e` | exits with a stack overflow (`0xC00000FD`) on the 5 cases whose answer would be `sat`; without `--mbqi` `unknown` |
| B2 fuel 1 and 2 | `unknown` on the same 5 cases | the same crash and `unknown` |

- The Z3 models of B1 replay (`numeric`, `skip`; `B-replay-b1.txt`), and
  Z3 with `smt.arith.solver=2` refutes `prefix` on `lt-e` with a model that
  replays; Accord doesn't set that option.
- cvc5 1.4.2 crashes whenever its answer on a B script would be `sat`, including
  `cmp_id` `lt` and `gt` of `numeric`, which don't use `fd`. The base shape
  doesn't crash. I didn't look for the cause.
- B2 never says `sat`, so no model exists to replay. For B2 the solvers' `unsat`
  rests on the axioms being true of the intended function; they are by
  construction, but I couldn't make a solver confirm they are satisfiable:
  `fd([0], [0, 1], 0) = 1` is `unknown` and `= 0` is `unsat` in Z3.
- In Z3, B2 fuel isn't a measured limit: with `smt.mbqi=false` (fuel by
  e-matching only) the result is the same as with MBQI.

What Accord's surface syntax would need for B (no compiler code written): a
recursive pure function usable in `requires` and `ensures`, with `decreases`
checked as a proof obligation of its own (not measured here, `fd` is one
`length(a) - k`); the contract names the function, so a library such as the
semver port would declare `fd` and `canon_from` beside the comparators; a
`requires` stated from an index (`canon_from(a, k)`) instead of for the whole
list; the compiler writing the fuel sort, `S`, the unfolding axiom, the
fuel-irrelevance axiom and the pattern for each such function (B2), or
`define-fun-rec` (B1); and a decision on how a counterexample over an
uninterpreted spec function is reported, since a B2 `sat` model can be
spurious.

## Part C: stability

Cases are the 36 obligations of the shape: 31 proofs (the contract's clauses,
bounds, overflow, `decreases` and calls' `requires`) and 5 open searches. 21
runs per case and solver. `results-round3/C-<shape>.txt` has every case.

| Shape | Z3 stable | Z3 unstable | Z3 unsolvable | cvc5 |
| --- | --- | --- | --- | --- |
| base, Z3 plain | 29 (2 slow) | 5 | 0 | 36 stable |
| base, Z3 `relevancy=1` | 33 (1 slow) | 2 | 0 | 36 stable |
| `TRIGGERS=1` | 23 | 0 | 13 | 36 stable |
| B1 | 35 | 0 | 1 | 36 stable |
| B2 fuel 1 | **36** | 0 | 0 | **36 stable** |
| B2 fuel 2 | **36** | 0 | 0 | **36 stable** |

Unstable cases of the base (unsat runs of 21): Z3 plain: `cmp_list` `gt-e` 15,
`lt-e` 16, `compare` `gt-e` 19, `compare_build` `gt-e` 13, `lt-e` 20; slow:
`compare` `lt-e` (5.4 s), `compare_build` open search (6.7 s). With
`relevancy=1`: `cmp_list` `gt-e` 18, `lt-e` 13; slow: `compare_build` `gt-e`
(9.4 s). The other runs of every unstable case were `unknown`. The unsolvable
cases of `TRIGGERS=1` are the 13 listed in part A, and B1's is the call's
`requires` of `cmp_id`. The slowest `unsat` of any run: B2 fuel 1 224 ms in Z3
and 207 ms in cvc5; fuel 2 358 and 141 ms; B1 166 and 144 ms.

The non-`unsat` runs of the base's unstable cases in Z3 plain, by kind of
variant: seeds 10 of 50, shuffles 7 of 25, renamings 6 of 25 (and the
unmodified case once). With `relevancy=1`: seeds 4, shuffles 2, renamings 5.
So a shuffle or a renaming alone changes Z3's answer on these cases, with
the same solver and default seed.

## Part D: bugs caught by running the contract

`python enumerate_inputs.py`, output in `results-round3/D-enumerate.txt`. The
alphabet of identifiers is canonical numeric (0, 1, 2, 10) and alphanumeric
(`a`, `b`, `1a`, `A`) values; lists have at most 3 elements over 8 identifiers
(1,277,201 inputs, every `k`) or at most 4 over 4 (512,713); versions have 3
main triples, pre-release at most 2 over 4 and build at most 2 over 3
(670,761 pairs). The correct comparator breaks no clause on any input, and
for every input the contract admits exactly the comparator's result (0 inputs
where it admits another or more than one).

| Function, input set | Fault | Inputs breaking a clause | Smallest | Clause | Time |
| --- | --- | --- | --- | --- | --- |
| `cmp_id`, 64 pairs | `numeric` | 32 | `1a` against `0` | `lt` | < 0.1 s |
| `cmp_list`, at most 3 over 8 | `numeric` | 506,656 | `[1a]` against `[0]` | `lt-e` | 3.3 s |
| | `prefix` | 86,288 | `[1a]` against `[]` | `lt-e` | 1.9 s |
| | `skip` | 78,912 | `[1a, 1a]` against `[1a]` | `eq` | 2.7 s |
| `cmp_list`, at most 4 over 4 | `numeric` | 239,080 | `[1a]` against `[0]` | `lt-e` | 1.7 s |
| | `prefix` | 61,000 | `[1a]` against `[]` | `lt-e` | 0.8 s |
| | `skip` | 75,920 | `[1a, 1a]` against `[1a]` | `eq` | 0.8 s |
| `compare`, 670,761 pairs | `numeric` | 117,624 | pre-releases `[1a]`, `[0]` | `lt-e` | 1.9 s |
| | `prefix` | 16,224 | `[1a, 1a]`, `[1a]` | `lt-e` | 1.5 s |
| | `skip` | 40,560 | `[1a, 1a]`, `[1a]` | `eq` | 1.3 s |
| `compare_build`, the same | `numeric` | 122,412 | pre-releases `[1a]`, `[0]` | `lt-e` | 1.6 s |
| | `prefix` | 18,870 | builds `[A]`, `[]` | `lt-e` | 1.1 s |
| | `skip` | 28,428 | pre-releases `[1a, 1a]`, `[1a]` | `eq` | 1.1 s |

Each fault is caught by the `e-versions` contract in every function that
calls the faulty one; `compare_main` calls none. `prefix` and `skip` change
only list comparison, so `cmp_id` has no violation for them. "Smallest" is by
the number of list elements, then in enumeration order. The times are of a
Python evaluator and include the contract's check of all three results for the
correct comparator; they were taken while solver runs shared the machine.

## What the third round shows

- **Spec functions with a fuel-limited axiom (B2) meet the proof and
  open-search part of the rule in both solvers and are stable.** All 36 cases
  are `unsat` in all 21 variants in both solvers, slowest run 0.36 s. With
  `define-fun-rec` (B1) Z3 misses one obligation in every variant.
- **No shape meets the whole rule.** B2 refutes `numeric` and `prefix` and
  `skip` in no solver: Z3 says `unknown`, cvc5 crashes on the `sat` answers.
  In the base shape with `smt.relevancy=1`, the proofs are met, but 2 of 36
  cases are unstable and 1 is slow; the faults are as in the second round.
- **Patterns hurt Z3 and don't change cvc5.** None of three designs helped, and
  the base proofs don't rest on patterns.
- **The base shape's second-round pass with `smt.relevancy=1` is a pass of the
  default seed.** Under 21 variants it keeps 2 unstable and 1 slow case, and plain
  Z3 keeps 5 unstable and 2 slow.
- **Running the contract on small inputs catches all three faults at
  every level in a few seconds,** with counterexamples of one or two list
  elements, and shows the correct comparator breaks no clause and the contract
  determines its result on every enumerated input. That doesn't depend on a
  solver.

## What the third round doesn't show

- The variants are mine, three trigger designs and two fuel values; others
  may do better or worse.
- The recursive spec function's termination obligation and `fd`'s own
  contract weren't generated or timed.
- The B2 axioms' satisfiability isn't confirmed by a solver; their truth is an
  argument.
- The stability runs have one run per variant. Z3 with 10 seeds, 5 shuffles and
  5 renames is a sample, not the distribution; classes are not stable under
  more variants. A case labelled stable in 21 runs may not be in 210.
- The enumeration is a finite domain; it shows no bug there, not none. The
  evaluator is mine, not Accord's interpreter.
- No Accord code ran, and cvc5's crash on B scripts was observed with the
  pinned cvc5 1.4.2 on Windows only.
- The times come from up to four processes at once on one machine; five alone
  runs of the cases near 5 s are in `results-round3/alone-*.txt`.

## Encoding for cvc5: axioms per sequence term

**Why.** cvc5 1.4.2 with `--mbqi` recurses without bound on any quantifier
over a `Seq` variable, which every B2 axiom has. On Windows it exits with a
stack overflow and no output; on Linux the stack grows about 0.7 GB per second
until the process is killed, past `--tlimit-per`. All 36 B2 proofs are closed
before MBQI runs, but sat answers and facts beyond the fuel hit it. Reported as
[cvc5/cvc5#13161](https://github.com/cvc5/cvc5/issues/13161), also seen on the
2026-11-13 nightly.

**Shapes.** [`ground.py`](ground.py) rewrites the B2 fuel-1 cases so each
axiom is written once per sequence term of the obligation, quantified only over
fuel and the index, and left out where no spec function is reached.
[`reveal.py`](reveal.py) then adds, for each spec call that the function's own
`requires` asserts, one ground step of its unfolding (a call-site reveal).

```sh
CONTRACT=e EQ=versions SPEC=b2f1 python gen.py <dir>/b2f1
python ground.py <dir>/b2f1 <dir>/ground
python reveal.py <dir>/ground <dir>/reveal
```

**Result.** Stability is 21 variants per case and solver, as in part C;
`results-round3/C-ground.*` and `C-reveal.*`.

| Shape | Z3 | cvc5 `--mbqi` | Faults (both solvers, Windows and Linux) |
| --- | --- | --- | --- |
| ground | 35 stable, 1 unstable | 36 stable | `numeric` sat; `prefix`, `skip` unknown; no crash |
| ground + reveal | 36 stable, slowest 134 ms | 36 stable | the same |

- The unstable case under `ground` is `cmp_list`'s call to `cmp_id`, whose
  `requires` needs one unfolding of `canon_from` at `k`: 2 of 5 shuffles were
  `unknown` in Z3. The reveal states that unfolding as a ground fact.
- On Linux (WSL Ubuntu, the pinned Linux binaries), the 48 `ground + reveal`
  cases give 43 `unsat` in both solvers, 2 `sat` in both, and 3 with Z3 at its
  limit and cvc5 `unknown`, as on Windows.
- A reveal is an instance of the unfolding axiom, so it adds no assumption.
- Without the per-term encoding, the cvc5 options `--mbqi-enum`,
  `--macros-quant`, `--inst-max-rounds`, `--mbqi-one-inst-per-round`,
  `--fmf-fun` and `--seq-array` still crash; running cvc5 without `--mbqi`
  avoids the crash but loses facts beyond the fuel.
- A spec function whose recursion changes a sequence argument gets no axiom for
  the new term, and its obligations end `unknown`. `fd` and `canon_from` only
  change the index.
