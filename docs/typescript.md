# TypeScript output

**Status: unstable until the first release.** `accord build <file or folder>
--out <folder>` writes a checked program as TypeScript modules that a project
imports with no runtime dependency.

## When it writes

`build` checks the whole program first, as `accord check` does. It writes only
when every obligation is proved and every `example` passes; otherwise it writes
nothing and exits 1 when something failed, or 3 when something wasn't proved.
An open case doesn't stop it: the module's header names each function with
one. A machine is written only when all its obligations are proved: one that is
`not_inductive` or undecided stops the build, like any other obligation.

| Code | Meaning |
| --- | --- |
| 0 | Every module written; each path is printed on stdout |
| 1 | An obligation refuted or an example failed; nothing written |
| 3 | An obligation undecided or unsupported; nothing written |
| 2 | Usage error |
| 4 | Source rejected, input or output unreadable, or solvers unavailable |

## What a module holds

Each Accord file becomes one module with the same base name: `gate.accord`
becomes `gate.ts`. In a folder, a module imports each enum its signatures use
from the module of the file that declares it, as `import type { … } from
"./<file>.js"`. The modules use only TypeScript that type stripping erases, so
Node runs them directly.

| Accord | TypeScript |
| --- | --- |
| `enum Light { Red, Amber }` | `export type Light = "red" \| "amber";`: each variant's `snake_case`, or the value it gives (`Red = "RED"`) |
| `pub fn` | An exported function, named in `camelCase`, with its parameters in `camelCase` and in order |
| `fn` without `pub` | Nothing, unless a function the module exports calls it |
| `Bool`, `Text` | `boolean`, `string` |
| `i32` | `number` |
| `i64` | `number` for the caller, `bigint` inside |
| `List<T>` | `readonly T[]` |
| `requires` | A check on entry |
| `pub machine M` | A type, a state check, and an exported function for each init, event and observer, below |
| `machine` without `pub` | Nothing |
| `ensures`, `open when`, `example`, `intent` | Nothing: they don't run |

Inside, a decision table is an `if` chain in row order. Where no row is
`otherwise`, the chain ends in a `throw` that proved coverage makes
unreachable. `and` and `or` are `&&` and `||`; both evaluation orders are
safe, since each operand is proved not to overflow whatever the other's value.
`in` is `.includes()`. Lists compare element by element. `all`, `any` and
`count` are `every`, `some` and `filter(…).length`, `length(list)` is
`list.length`, and both are `bigint`s, since they are `i64`s. An index a
quantifier ranges over is the callback's `number`, read as a `bigint`
except where it indexes an array. `list[k]` is `list[k]!`: every index is
proved within its list, so a module typechecks under
`noUncheckedIndexedAccess`.

A call runs the callee's body directly, with no boundary check, since each
call's `requires` is proved. Every function a module's exported functions call,
directly or not, is written into that module unexported, even when another
file declares it: no module exports a body without its checks, and modules
never import each other's functions.

`i32` arithmetic in `number` is exact: every operation's result is proved to
fit `i32`, so it is well inside the integers a `number` holds exactly. `i64`
arithmetic runs in `bigint`.

## Machines

A `pub machine Switch` becomes `export type Switch = { readonly light: Light;
readonly presses: number; }`, its fields in order and in `camelCase`, and
a function `checkSwitch$` that no module exports. Each init, event and
observer is an exported function named in `camelCase`, with its body unexported
beside it, as for a function:

- an init takes its parameters and returns the state it gives;
- an event takes the state, then its parameters, and returns the next state,
  a new object that lists every field; it leaves the state it was given
  unchanged;
- an observer takes the state, then its parameters, and returns its result.

An `i64` field is a `number` in the state, and an event that would leave one
that isn't a safe integer throws a `RangeError`, as a function does for its
result. A module whose exported names, or their bodies' names, would collide is
refused, and nothing is written.

## The boundary

The proofs assume the caller is checked code. A TypeScript caller isn't, so
each exported function checks, in order, before it runs:

| What | Error |
| --- | --- |
| A `Bool` is a `boolean`; a number is a `number`; a text is a `string`; an enum value is one of its variants; a list is an array of such items | `TypeError` |
| An `i32` is an integer in range; an `i64` is a safe integer (`Number.isSafeInteger`) | `RangeError` |
| A text holds only characters up to U+2FFFF, the solvers' alphabet, with no lone surrogates | `RangeError` |
| Each `requires` holds, evaluated with mathematical integers (`bigint`) | `RangeError`, naming the clause |

An event or observer first checks the state it is given, before its own
parameters: it is an object (`TypeError`), each field has its type and range
(`TypeError`, `RangeError`, worded `state.<field> must be …`), and then it
meets every invariant, then every helper invariant, evaluated with
mathematical integers (`RangeError`, `state breaks invariant <name>: …`). An
init has no state to check; its result meets the invariants by proof.

An `i64` result that isn't a safe integer throws a `RangeError` instead of
losing precision. So the generated function accepts fewer inputs than the
proofs cover, and never runs on one they don't.

## What it trusts

A module is not proved equivalent to the Accord program it comes from. The
compiler's own tests compare each module with the interpreter: on every
combination of `Bool` and enum values, each integer width's boundaries and a
seeded sample, the program's text literals and characters beyond ASCII, each
`example`'s call and every input a solver gave, with a seeded sample past
4,096 combinations per function. For a machine, the same inputs for each
init, event and observer, and for events and observers, states built from its
fields' values and the integers its clauses name, most of them breaking an
invariant or helper invariant, which the module must reject. Agreement on those inputs is evidence, not a
proof.
