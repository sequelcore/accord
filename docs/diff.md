# Comparing contracts: `accord diff`

**Status:** version 1, unstable until Accord's first release, like
[the receipt](receipt.md).

```sh
accord diff <old> <new> [--json]
git show main:rule.accord | accord diff - rule.accord
```

`accord diff` compares the contracts of two revisions of a file and says,
for each function, whether the new contract is weaker, stronger or the same.
It's for a reviewer, and for a check that an agent didn't make a failing
proof pass by loosening what was promised. Either file may be `-` for stdin;
not both.

## Weaker and stronger

A function's contract is its `requires` R and its `ensures` E. The new
contract is **at least as strong** as the old when it accepts every input the
old one did (`R_old ⇒ R_new`) and, on those inputs, promises everything the
old one did (`R_old ∧ E_new ⇒ E_old`). The solvers decide that and the
reverse, under the same rules as any obligation: both must agree, and every
counterexample is replayed by the interpreter.

| Change | Meaning |
| --- | --- |
| `equivalent` | The same contract, in meaning, however it is written |
| `stronger` | It accepts more inputs or promises more, and nothing less |
| `weaker` | It accepts fewer inputs or promises less, with what it newly allows |
| `incomparable` | Each is stronger somewhere; reported with where the new one loosened |
| `undecided` | The solvers didn't settle it, with the reason |
| `added` | Only in the new revision |
| `removed` | Only in the old revision: its guarantees are gone |
| `renamed` | The same definition under a new name |
| `signature_changed` | Its parameter or result types changed, so the contracts can't be compared |
| `no_contract` | Neither revision has a `requires` or an `ensures` |

A loosening is reported as the input the new contract no longer accepts, or
as an input and a result the new contract allows and the old one ruled out.
Functions are matched by name; parameters are compared by position, so
renaming one changes nothing. `diff` compares two files, each on its own: a
file that uses declarations from its folder can't be compared yet. Only contracts are compared: a decision table's
behaviour is the body, and a change to it changes the function's
[identity](concepts.md), not its contract.

## Exit code

| Code | Meaning |
| --- | --- |
| 0 | No contract got weaker |
| 1 | A contract got weaker or incomparable, or a function was removed |
| 3 | Nothing was proved weaker, but something couldn't be compared |
| 2 | A usage error, such as both files given as `-`, or a folder given instead of a file |
| 4 | A file couldn't be read or was rejected (named with its line), or the solvers couldn't be found |

## JSON, version 1

```json
{
  "comparison": 1,
  "accord": "0.0.0",
  "old": "old.accord",
  "new": "new.accord",
  "verdict": "weakened",
  "functions": [
    {
      "name": "add",
      "change": "weaker",
      "loosening": {
        "allows": {
          "inputs": [{ "name": "x", "value": "1" }, { "name": "y", "value": "0" }],
          "result": "2"
        }
      }
    }
  ]
}
```

`verdict` is `kept`, `weakened` or `undecided`. A `weaker` or `incomparable`
function has `loosening`, with either `rejects` (an input array) or `allows`
(`inputs` and `result`); `undecided` has `reason`; `renamed` has `was`, the
old name. Object keys come in no fixed order; arrays keep source order.
