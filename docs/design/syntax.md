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
| `enum Name { A, B, … }` | A type whose values are exactly the listed variants. A variant may give its value outside Accord as a string, `A = "A_VALUE"`; otherwise it is its name in `snake_case`. No two variants of an enum share a value |
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
  contain `"`; any other character, accented or not, stands for itself, up
  to U+2FFFF, the last character the solvers reason about. A literal with a
  later one is rejected. Texts are compared with `==` and `!=` and have no
  other operations yet.
- `List<T>`: a sequence of values of type `T`, written `["read", "grep"]`
  or `[]`. `T` is `Bool`, an enum or `Text`; lists of integers wait for a
  target that needs them. A list literal takes its element type from its
  context where its elements don't give it, as `[]` always does. Lists are
  compared with `==` and `!=`, and `x in list` asks whether `x` is one of the
  elements; it binds like a comparison, so `not tool in readers` needs no
  parentheses. `length(list)` is an `i64`, and `list[k]` is the element at
  index `k`, an `i64` from 0, which must be in range: that is an obligation
  wherever the index is evaluated, in code or in an `ensures`. A `requires`
  or `open when` can't index a list yet. Every list is assumed to hold at
  most 4,294,967,295 elements, as a TypeScript array does.
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
  qualified (`ProofOutcome.Passed`) where it isn't. The canonical form uses
  the bare variant wherever it can, so a qualified variant appears only where
  it is needed.

## Expressions

| Kind | Forms |
| --- | --- |
| Logic | `and`, `or`, `not`; in `requires` and `ensures`, also `implies` |
| Comparison | `==`, `!=` on any type; `<`, `<=`, `>`, `>=` on integers; `in` from an element to a list |
| Arithmetic | `+`, `-`, `*`, and `-` before a value; `/` and `%` truncate toward zero (`-7 / 2 == -3`, `-7 % 2 == -1`), in code and in specifications, with a zero divisor and `MIN / -1` obligations. `/` and `%` aren't compiled yet |
| Choice | `if condition { a } else { b }`, always with `else` |
| Quantifier | `all x in list: condition`, `any x in list: condition` and `count x in list: condition`, by element; `all k in indices(list): condition` and the same for `any` and `count`, by index. `all` and `any` are `Bool`, `count` an `i64`. The condition extends as far as an expression does, so `(all x in xs: x) and ready` needs its parentheses. The name can't be a parameter's, and `count` can't be inside another quantifier yet |
| Length and index | `length(list)`, `list[k]` |
| Call | `name(label: value, …)`; labels are required when the function has more than one parameter. A body or a decision table can call any function of its program; a specification and an example's arguments can't yet. A function can't call itself, directly or through others |

A call is proved through the callee's contract, never its body: the caller
must meet the callee's `requires` wherever the call runs, and knows of the
result only what the callee's `ensures` say. Changing a callee's body alone
leaves its callers' proofs current; changing its contract doesn't.

Blocks are expressions: the last expression is the block's value. There is no
`return`: an early exit is a decision table.

## Machines

Accepted for stage 10 ([#82](https://github.com/sequelcore/accord/issues/82)).
`accord check` checks a machine; `accord build` doesn't write it as
TypeScript yet. A machine is a state, the ways it starts, the events
that move it, and what must hold in every state it reaches:

```accord
/// A failed model call, retried within a budget and a time window.
pub machine ProviderRetry {
    intent "Retry a failed model call at most three times within its window, and never call again after cancellation."

    state {
        phase: RetryPhase,
        retries: i32,
        calls: i32,
        stopped: Bool,
        calls_at_stop: i32,
    }

    init start() {
        next {
            phase: Failed,
            retries: 0,
            calls: 1,
            stopped: false,
            calls_at_stop: 0,
        }
    }

    event schedule()
        requires phase == Failed
    decide {
        | not stopped and retries < 3 => next { phase: Waiting, retries: retries + 1 }
        | otherwise                   => next { phase: Ended }
    }

    event start_call()
        requires phase == Waiting and not stopped
    {
        next { phase: Calling, calls: calls + 1 }
    }

    event cancel()
        requires not stopped
    {
        next { stopped: true, calls_at_stop: calls }
    }

    invariant cancel_stops_calls: stopped implies calls == calls_at_stop

    helper invariant calls_follow_retries: calls <= retries + 1
}
```

| Form | Meaning |
| --- | --- |
| `machine Name { … }` | A `state`, at least one `init`, its events, observers, invariants and helper invariants. `pub` exports it to TypeScript. Its `intent` is about the machine |
| `state { field: Type, … }` | The fields: fixed-width integers, enums and `Bool`s |
| `init name(params) clauses body` | A way to start. Its `next { … }` lists every field |
| `event name(params) clauses body` | A transition. Its `requires` say when it is allowed and which parameters it takes |
| `next { field: value, … }` | The state after the event. A field it doesn't list keeps its value, and every expression in an `init` or event, conditions and observer calls included, reads the state before it. `next { }` changes nothing |
| `fn name(params) -> Type` inside a machine | An observer: a pure function of the state, with the usual clauses. Events and observers can call it; an invariant can't, as no specification can call a function |
| `invariant name: condition` | A claim about every reachable state: proved inductive with the helper invariants, or refuted by a trace from an `init` |
| `helper invariant name: condition` | A condition proved with the invariants so they can be proved inductive. The receipt reports it as support, not as a claim, and the boundary checks it like an invariant |

- An `init` or event takes `intent` and `requires` only; it has no `result`.
  Its body is a block that ends in `next { … }`, through `if`, which always
  has an `else`, or a decision table whose rows give `next { … }`.
- A parameter can't share a field's name, and `next` is reserved, so no
  field or parameter can use it.
- Each event is proved to be allowed in some reachable state, as each row of a
  decision table is proved reachable.
- A machine's example names it, its events one per line, and what holds after:

  ```accord
  example ProviderRetry
      start()
      schedule()
      start_call()
      gives phase == Calling and calls == 2
  ```

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
- One canonical layout, as above: four-space indentation, a parameter per
  line when they don't fit on one, aligned arrows in decision tables,
  trailing commas in multi-line lists.

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

Accepted on 2026-11-12 ([#82](https://github.com/sequelcore/accord/issues/82)):

7. **An event states its next state as `next { field: value }`**, and every
   expression in it reads the state before the event. In a
   [probe](probes/2026-11-12-machine-updates/README.md), every reader read
   that form so, while seven readers of eight read `field = value`, one
   update per line, as updating in order, and one read `field' = value` so.
   A program that reads a field after setting it means, in `next { … }`,
   what its readers expect. `helper invariant` keeps what supports a proof
   apart from what the machine claims, and every reader told the two apart.
