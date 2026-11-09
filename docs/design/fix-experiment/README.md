# E4. Fixing a failed proof

**Status:** registered on 2026-11-08, before any run. The tasks are built at
the end of stage 2, before the first run, from the compiler output of that
time. The [roadmap](../../../ROADMAP.md) lists it with the other experiments.

## Question

Can a developer who has never seen Accord turn a failed check into a
verified fix that keeps the function's intent, with only the compiler's
output and the [syntax reference](../syntax.md)? The
[comprehension check](../comprehension-check/README.md) asks whether people
can read Accord; this asks whether they can repair it, which is where a
verifier usually costs the most time.

## Material

Six tasks, each a function with a one-sentence intent and exactly one failed
obligation, in random order per participant:

| Task | What fails | An honest fix |
| --- | --- | --- |
| T1 | Overflow in code that runs | A `requires` that rules the input out, a wider type, or a guard |
| T2 | A decision table that no row covers for some input | A row for the uncovered input |
| T3 | A row no input reaches | Reordering or removing the shadowed row, as the intent says |
| T4 | An `ensures` refuted by a wrong body | Correcting the body |
| T5 | An `ensures` refuted because the specification is wrong | Correcting the `ensures` to the intent |
| T6 | A refutation reached through a `requires` that is too weak | Strengthening the `requires` only as far as the intent allows |

Each task also has hidden examples: concrete calls and the results the intent
gives. They aren't shown to participants.

## Procedure

At least five developers who haven't used Dafny, Verus or Lean. Each runs
`accord check` on their own, with the syntax reference open, for at most 15
minutes per task. Recorded per task: the time to the first run that exits 0,
the number of runs, and the final file.

## Scoring

A task is **solved** when, within 15 minutes, `accord check` exits 0 and the
fix is **honest**: every hidden example still gives its result, and the fix
doesn't make the obligation pass by weakening what was promised (removing or
weakening an `ensures`, or a `requires` that rules out inputs the intent
covers). Hidden examples are checked by running them; the second condition is
judged by hand against the table above, with each judgment written beside
the final file.

A verified fix that isn't honest is counted and reported on its own: it is
the failure a verifier invites.

## Pass mark

For the run at the end of stage 2:

- at least **70%** of all task attempts solved;
- every task solved by at least **half** of the participants, so no kind of
  failure is systematically unfixable from the output;
- no more than **10%** of verified fixes dishonest.

The second run, at stage 7, uses new participants and the review view on the
same tasks. It passes when the solved rate doesn't fall and the median time to
a solved task falls. The pass marks don't change after a run.
