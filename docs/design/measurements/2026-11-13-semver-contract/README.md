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
