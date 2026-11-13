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

Accord at `11c5482`, on `docs/stage-11-decisions`. Z3 5.3.3 and cvc5 1.4.2
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
