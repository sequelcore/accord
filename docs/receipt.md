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
| `..outcome` | `proved`, `refuted`, `undecided` or `unsupported` |
| `..witness` | With `proved`, for an obligation that needs some input: that input |
| `..counterexample` | With `refuted`: inputs on which it fails |
| `..no_such_input` | With `refuted`, `true` when the obligation needs some input and both solvers proved none exists |
| `..reason` | With `undecided` or `unsupported`: why |
| `.examples[]` | One per `example` that calls the function, in source order |
| `..text`, `..line` | The example as written, on one line, and its line |
| `..run` | `passed` or `failed`; with `failed`, `reason` says why |

Inputs (`witness`, `counterexample`) are arrays of `{"name": ..., "value": ...}`
in parameter order, with values written as Accord source (`-3`,
`Light.Amber`).

## Standing

A function is **failed** if an obligation is refuted or an example fails;
otherwise **incomplete** if an obligation is undecided or unsupported;
otherwise **verified** if it has any obligation or example; otherwise
**unverified**. The file is failed if any function is, verified only if every
function is, unverified if none has evidence, and incomplete otherwise.

## Not in version 1

The [concepts](concepts.md) describe a receipt that also carries freshness
(current or stale, from stage 3's ledger), the solver versions, assumptions,
opinions kept apart, and what wasn't checked. None of these exist in the
compiler yet; each joins the receipt when it does.
