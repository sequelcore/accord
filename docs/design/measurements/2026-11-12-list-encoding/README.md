# Encoding lists for quantifiers: solver answers, 2026-11-12

**Status:** two runs of small hand-written obligations on one machine, the
second after calls between functions existed. They inform stage 9's encoding
of lists ([#69](https://github.com/sequelcore/accord/issues/69));
it doesn't measure the obligations Accord will generate.

## Question

Stage 9 adds length, indexing, and `all`, `any` and `count` over lists. Which
SMT-LIB encoding lets both pinned solvers prove such obligations and refute
false ones, as a planted fault needs?

## What runs

[`run.sh`](run.sh) runs every file in [`cases/`](cases) with Z3, cvc5, and
cvc5 with `--mbqi`, 10 seconds each. Each file asserts the negation of a
claim: `unsat` proves it, `sat` refutes it. The cases are shaped like
Tesota's `verdict-rule`, with elements of type `Bool`, an enum (`Standing`)
and `Text` (`String`):

| Case | Claim |
| --- | --- |
| `*-parallel-prove` | `runStanding`'s shape: three lists of equal length, where "not every path is covered" equals "some path isn't" |
| `*-contains-mixed-prove` | "every element differs from `v`" implies "`v` is not in the list", with `in` as `seq.contains` |
| `seq-Standing-in-as-index-prove` | The same claim, with `in` as "some index holds `v`" |
| `*-refute` | A false claim about a quantified list: every element is `v`, so the list has at most one element |
| `array-*` | The prove and refute cases, with a list as an array and a separate length |
| `count-recursive-*` | `count` defined recursively: its bounds and its zero case, and a false bound |
| `count-axioms-prove` | `decisions`' contract from `count`'s axioms. It only shows that the axioms suffice, which is immediate |
| `call-in-all-*` | Second run. `runStanding`'s covered-paths clause, with `all k` calling `path_covered` at each index: proved, refuted by a false clause, and refuted where `path_covered`'s contract is too weak. `-global` states the callee's contract once, for every application; `-site` states it at the call, under the same `k` |
| `seq-Standing-any-prove` | Second run. `partlyHolds`' shape: some standing is `Decide` or `Gap`, as `exists`, equals not every one being neither |
| `index-in-range-prove` | Second run. An index bounded by its guard is in range, as an indexing obligation needs |

Accord's whole test suite also ran three times at `202fef0`: with the pinned
solvers; with cvc5 through a wrapper script that adds `--mbqi`; and through
the same wrapper without it, to separate the cost of the wrapper.

## Identity

Accord at `202fef0` for the first run and `a11e27a` for the second. Z3 5.3.3 and cvc5 1.4.2 from
`scripts/install-solvers.sh`. Windows 11 Pro, a developer machine whose
background load wasn't controlled.

## Result

| Case | Z3 | cvc5 | cvc5 `--mbqi` |
| --- | --- | --- | --- |
| `seq-{Bool,Standing,String}-parallel-prove` | unsat | unsat | unsat |
| `seq-{Bool,Standing,String}-contains-mixed-prove` | timeout | unknown | timeout |
| `seq-Standing-in-as-index-prove` | unsat | unsat | unsat |
| `seq-{Bool,Standing,String}-refute` | sat | unknown | sat |
| `array-{Bool,Standing,String}-prove` | unsat | unsat | unsat |
| `array-{Bool,String}-refute` | sat | unknown | sat |
| `array-Standing-refute` | sat | unknown | unknown |
| `count-recursive-prove` | timeout | timeout | timeout |
| `count-recursive-refute` | sat | unknown | timeout |
| `count-axioms-prove` | unsat | unsat | unsat |
| `call-in-all-prove-{global,site}` | unsat | unsat | unsat |
| `call-in-all-refute-{global,site}` | sat | unknown | sat |
| `call-in-all-weak-{global,site}` | sat | unknown | sat |
| `seq-Standing-any-prove` | unsat | unsat | unsat |
| `index-in-range-prove` | unsat | unsat | unsat |

The test suite passed in all three runs, every outcome unchanged: 57 s with
the pinned solvers, 84 s through the wrapper without `--mbqi`, 86 s with it.
The wrapper, not `--mbqi`, accounts for the difference.

## What it shows

- **One encoding for `in` and quantifiers.** Where `in` is `seq.contains` and
  a quantifier reads elements by index, neither solver connects the two. With
  `in` over indices, both prove the claim.
- **cvc5 refutes a quantified claim only with `--mbqi`.** Accord counts a
  refutation only when both solvers find an input, so without it every
  planted fault with a quantifier would be undecided. With `--mbqi`, the
  sequence encoding is refuted for all three element types; the array
  encoding isn't, for the enum.
- **`count` needs axioms.** Neither solver decides the recursive definition's
  bounds, which need induction.
- **A call under a quantifier is decided** in both encodings of the
  callee's contract, and refuted with `--mbqi`, as above. Where the
  contract is too weak, the solvers' input rests on the callee's value at
  each index, which a model doesn't list; replaying it needs those values.
- **`seq.len` over `Seq String`** was decided in these cases, unlike the case
  [#58](https://github.com/sequelcore/accord/issues/58) records. That case is
  still unexplained.

## What it doesn't show

- The obligations Accord will generate, or `verdict-rule` as a whole.
- Answer times: each case ran once, with a 10-second limit.
- That `--mbqi` keeps every future result: the suite has no quantifier yet.
