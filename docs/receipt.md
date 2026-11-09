# The receipt

**Status:** version 1, unstable. The format may change in any version until
Accord's first release; the `receipt` field says which version a receipt
follows. The [concepts](concepts.md) define each term.

`accord check <file>` writes the receipt as text, and `accord check <file>
--json` as JSON. Both carry the same record: every obligation's outcome,
every example's run, each function's standing and the file's.

## Exit code

| Code | Standing of the file |
| --- | --- |
| 0 | verified |
| 1 | failed: an obligation was refuted or an example failed |
| 3 | incomplete or unverified: nothing failed, but something wasn't proved, or nothing was checked |
| 2 | a usage error |
| 4 | the file was rejected or couldn't be read, or the solvers couldn't be found; no receipt |

## JSON, version 1

Object keys come in no fixed order; arrays keep source order.

```json
{
  "receipt": 1,
  "accord": "0.0.0",
  "file": "conformance/cases/example-requires-fault.accord",
  "standing": "failed",
  "definitions": [
    {
      "name": "count",
      "kind": "function",
      "hash": "98d964c8…",
      "standing": "failed",
      "obligations": [
        {
          "clause": "ensures result == (n > 0)",
          "line": 4,
          "key": "2e0445ed…",
          "reused": false,
          "outcome": "proved"
        }
      ],
      "examples": [
        {
          "text": "example count(n: -1) == true",
          "line": 9,
          "run": "failed",
          "reason": "the call breaks `requires n >= 0`"
        }
      ]
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `receipt` | The schema version: `1` |
| `accord` | The compiler's version |
| `file` | The file as given on the command line |
| `standing` | The file's: `verified`, `incomplete`, `failed` or `unverified` |
| `definitions[]` | One per function, in source order |
| `.name`, `.kind` | The function's name; `kind` is `function` |
| `.hash` | SHA-256 of the normalized function: its identity, which no rename changes |
| `.standing` | The function's, from its own obligations and examples |
| `.obligations[]` | One per obligation, in source order |
| `..clause`, `..line` | The obligation as written and its line |
| `..key` | SHA-256 of the obligation's SMT-LIB script: exactly what was checked |
| `..reused` | `true` when the outcome came from the ledger instead of the solvers |
| `..outcome` | `proved`, `refuted`, `undecided` or `unsupported` |
| `..witness` | With `proved`, for an obligation that needs some input: that input |
| `..counterexample` | With `refuted`: inputs on which it fails |
| `..no_such_input` | With `refuted`, `true` when the obligation needs some input and both solvers proved none exists |
| `..reason` | With `undecided` or `unsupported`: why |
| `.examples[]` | One per `example` that calls the function, in source order |
| `..text`, `..line` | The example as written, on one line, and its line |
| `..run` | `passed` or `failed`; with `failed`, `reason` says why |
| `.strength` | `null`, or how the contract fared against mutants of the body (below) |

Inputs (`witness`, `counterexample`) are arrays of `{"name": ..., "value": ...}`
in parameter order, with values written as Accord source (`-3`,
`Light.Amber`).

## Strength

`accord check` also plants mutants in the body of every function with an
`ensures`: comparisons and boundaries changed (`<` and `<=`, `==` and `!=`,
a literal by one), connectives swapped (`and` and `or`), conditions
negated, and in decision tables adjacent rows swapped, a row's result
changed or a row removed while `otherwise` keeps the table complete. Each
mutant is checked against the function's own contract; see **strength** in
the [concepts](concepts.md). At most 64 are tried per function.

The text receipt shows each surviving mutant as `weak`, with the input on
which the code and the change differ, and a `strength` line per function.
In JSON, `strength` holds `tried`, `caught`, `equivalent`, `undecided`,
`not_tried` and `survivors[]`, each with `change`, `inputs`, `original` and
`mutant`. A weak contract doesn't change standing.

## The ledger

`accord check` keeps a ledger in `.accord/` beside the file it checks: a
local cache, not meant to be committed. An obligation whose script was
already decided by the same Z3, cvc5 and compiler build is **reused**, not
decided again; the text receipt marks it `[reused]` and its summary counts
them. A changed body or contract changes its scripts, and an enum change
changes the scripts of the functions that use it, so exactly those are
decided again; a rename changes no script.

Only `proved` and `refuted` are stored. An undecided obligation is decided
again on every run, and an entry that can't be read is decided again, never
trusted. Examples always run. Deleting `.accord/` loses nothing but time.

## Standing

A function is **failed** if an obligation is refuted or an example fails;
otherwise **incomplete** if an obligation is undecided or unsupported;
otherwise **verified** if it has any obligation or example; otherwise
**unverified**. The file is failed if any function is, verified only if every
function is, unverified if none has evidence, and incomplete otherwise.

## Not in version 1

The [concepts](concepts.md) describe a receipt that also carries the solver
versions, assumptions, opinions kept apart, and what wasn't checked. None of
these exist in the compiler yet; each joins the receipt when it does. Stale
evidence never appears: a key that no longer matches is simply not found, and
its obligation is decided again.
