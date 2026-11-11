# Syntax draft

**Status:** draft for stage 1 ([#2](https://github.com/sequelcore/accord/issues/2)).
It covers what stage 2 compiles: enums, pure functions, `intent`,
`requires`, `ensures`, decision tables and examples, over `Bool` and
fixed-width integers. The decisions taken on it are listed at the end. The
[concepts](../concepts.md) define the terms and the
[influences](influences.md) state the design goals each choice serves.

## A complete example

Tesota's `proof-outcome-rule`, as Accord:

```accord
/// How one proof of a file ended.
pub enum ProofOutcome { Passed, Failed, Vacuous, TimedOut, Cancelled, NotStarted }

pub fn proof_outcome(
    cancelled: Bool,
    timed_out: Bool,
    started: Bool,
    regenerated: Bool,
    check_exit: i32,
    verified: i64,
) -> ProofOutcome
    intent "A proof passes only if nothing stopped it and something was verified."
    requires verified >= 0
decide {
    | cancelled                          => Cancelled
    | timed_out                          => TimedOut
    | not started                        => NotStarted
    | not regenerated or check_exit != 0 => Failed
    | verified > 0                       => Passed
    | otherwise                          => Vacuous
}

example proof_outcome(
    cancelled: false, timed_out: false, started: true,
    regenerated: true, check_exit: 0, verified: 3,
) == Passed

example proof_outcome(
    cancelled: false, timed_out: true, started: true,
    regenerated: true, check_exit: 0, verified: 3,
) == TimedOut
```

The compiler proves that every input reaches a row, that no row is
unreachable, and that both examples hold. The second example states the case
the [probe](probes/2026-11-07-intent-and-decision-tables/README.md) planted
as an error.

The same function with a body and a contract, for rules that aren't
decisions:

```accord
/// The larger of two counts.
pub fn larger(a: i64, b: i64) -> i64
    intent "Returns whichever count is larger."
    ensures result >= a and result >= b
    ensures result == a or result == b
{
    if a >= b { a } else { b }
}

example larger(a: 2, b: 5) == 5
```

## Declarations

| Form | Meaning |
| --- | --- |
| `enum Name { A, B, … }` | A type whose values are exactly the listed variants |
| `fn name(params) -> Type clauses body` | A pure, total function |
| `example call == value` | A concrete case, checked by running it |
| `pub` before a function | Exported from its file's [TypeScript module](../typescript.md); within a folder every file sees every declaration either way |
| `///` before a declaration | Documentation for people; never checked |

A function's **clauses** come between its signature and its body, in this
order: `intent` (at most one), `requires`, `ensures`, `open when`. Its **body** is either
a block `{ … }` or a decision table `decide { … }`.

## Decision tables

```accord
decide {
    | condition => result
    | otherwise => result
}
```

Rows are tried from the top; the first whose condition holds gives the
result. `otherwise` matches anything and, when present, is the last row. The
compiler proves that every input meets some row (so `otherwise` is needed
only when the rows above don't cover every input), and reports any row whose
condition can never be the first to hold.

## Clauses

- `intent "…"` states what the function is for, in plain language. Nothing
  checks it. It belongs to the function it's written in.
- `requires condition` is a condition callers must meet. Every call is an
  obligation to prove it.
- `ensures condition` is a guarantee about `result`, the value the function
  returns. The body is an obligation to meet it.
- `open when condition` declares a **permitted freedom**: for inputs where
  the condition holds, any result the contract allows is acceptable. It
  describes inputs, so it can't mention `result`, and like `requires` it is
  a specification, with mathematical integers. A function may have several;
  an input is covered when any of them holds. The compiler reports an **open
  case**, an input for which the contract allows two different results, only
  where no `open when` covers it, and the receipt lists every freedom
  declared, so an accepted freedom is never silent:

  ```accord
  fn positive(n: i64) -> Bool
      ensures result implies n >= 0
      open when n >= 0
  {
      n > 0
  }
  ```

## Types and values

- `Bool`: `true`, `false`.
- `Text`: a sequence of characters, written in double quotes (`"read"`). A
  literal ends at the next `"` on its line and has no escapes, so it can't
  contain `"`; any other character, accented or not, stands for itself.
  Texts are compared with `==` and `!=` and have no other operations yet.
- `List<T>`: a sequence of values of type `T`, written `["read", "grep"]`
  or `[]`. `T` is `Bool`, an enum or `Text`; lists of integers wait for a
  target that needs them. A list literal takes its element type from its
  context where its elements don't give it, as `[]` always does. Lists are
  compared with `==` and `!=`, and `x in list` asks whether `x` is one of the
  elements; it binds like a comparison, so `not tool in readers` needs no
  parentheses. There is no indexing or quantifier over a list yet.
- `i32`, `i64`: signed integers of that width. Arithmetic that could leave the
  range is an obligation; the program can't overflow. The obligation holds
  under the conditions that make the operation run: only the branch of an
  `if` that runs, and only the decision table row that decides. Both
  operands of `and` and `or` must stay in range, whatever the other's value.
- An integer literal takes its type from its context, as a bare variant
  does (`n > 0`); in code that runs it must fit that type.
- In `requires` and `ensures`, integer arithmetic is mathematical: it never
  overflows, so a specification can state `a + b > a` without caring about
  width, and its literals needn't fit a width.
- Enum variants are written bare (`Passed`) where the type is known, and
  qualified (`ProofOutcome.Passed`) where it isn't. The formatter writes the
  bare form wherever it can, so a qualified variant appears only where it is
  needed.

## Expressions

| Kind | Forms |
| --- | --- |
| Logic | `and`, `or`, `not`; in `requires` and `ensures`, also `implies` |
| Comparison | `==`, `!=` on any type; `<`, `<=`, `>`, `>=` on integers; `in` from an element to a list |
| Arithmetic | `+`, `-`, `*`, and `-` before a value; `/` and `%` truncate toward zero (`-7 / 2 == -3`, `-7 % 2 == -1`), in code and in specifications, with a zero divisor and `MIN / -1` obligations. `/` and `%` aren't compiled yet |
| Choice | `if condition { a } else { b }`, always with `else` |
| Call | `name(label: value, …)`; labels are required when the function has more than one parameter |

Blocks are expressions: the last expression is the block's value. There is no
`return`: an early exit is a decision table.

## Files and folders

A file is a program on its own. A folder is one program too: every `.accord`
file directly in it sees every other's declarations, with no import syntax,
and a name declared in two of them is an error. A subfolder is a program of
its own. `accord check <file>` checks the file alone; `accord check
<folder>` checks the folder, and reports each definition with its file. A
file that uses a declaration from its folder is checked through its folder.

## Names and layout

- Functions, parameters and values are `snake_case`; types and variants are
  `PascalCase`.
- One canonical layout, applied by the formatter, as above: four-space
  indentation, a parameter per line when they don't fit on one, aligned
  arrows in decision tables, trailing commas in multi-line lists.

## Decisions

Accepted on 2026-11-07:

1. **Specifications use `implies` and no equivalence operator.** An
   equivalence is written as two `implies`, or as a decision table. Every
   model in the probe found `<==>` the hardest symbol, and equivalences are
   what decision tables replace.
2. **Labels are required at call sites when a function has more than one
   parameter**, so `proof_outcome(false, true, true, true, 0, 3)` can't be
   written.
3. **`intent` is a clause inside the function**, so it can't drift away from
   the function it describes.
4. **`result` is a fixed name** for the value a function returns. Every model
   in both probes understood it.
5. **Variants are bare where the type is known.** Version 2 of the
   [comprehension check](comprehension-check/v2/README.md) compared bare,
   always-qualified and leading-dot variants; all three passed and read
   equally well, including two enums that share a variant name. The bare form
   is the shortest, and decision tables read most like their rule.

Accepted on 2026-11-08 ([#27](https://github.com/sequelcore/accord/issues/27)):

6. **A permitted freedom is the clause `open when condition`**, after
   `ensures`, so it sits beside the contract it relaxes and can't drift from
   it, for the same reason `intent` is a clause. An open case it doesn't cover makes the function
   incomplete, not failed: whether a freedom is intended is a person's
   decision.
