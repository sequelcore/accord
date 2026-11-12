# The receipt and review view

**Status: version 3, unstable until the first release.** `accord check <file or
folder>` produces a per-function terminal review view. Add `--json` for the
same facts as a structured record with stable diagnostic codes. Both report
only checks actually performed; neither records human approval.

[Concepts](concepts.md) defines the terms. The
[human readability check](design/stage-7-review/README.md) contains generated
packets for the whole gate rule and Cedar. Human acceptance is separate from
compiler and material checks. [E2](design/e2-human-review/README.md) remains a
draft, not a registered or completed experiment.

## What each function shows

The view contains its signature, used enum variants, unchecked intent, and every
specification clause as written, including requires and declared freedoms.
It then shows every obligation, example run, open-case result, mutation result,
and an explicit list of what was not proved. A missing intent, absent examples,
no postcondition, or an analysis that did not run is explicit.

The signature and clauses describe the admitted domain and guarantees without
requiring the implementation body. Text describes row reachability as an
existential check, with a concrete input when proved, rather than printing the
row's implementation. JSON retains the source clause for precise diagnostics.
An overflow finding or surviving mutant can still contain the relevant operation
or change: those are evidence, not a complete body listing.

Intent is neither evidence nor approval. A contradictory intent does not refute
a contract; a function with only intent and no checked obligations or examples
is unverified. A declared freedom is not silently described as accepted by a
person. Determined means one permitted result outside declared freedoms; it does
not establish that the result is intended.

## Exit code and streams

| Code | Meaning |
| --- | --- |
| 0 | Checked program is verified |
| 1 | Checked program failed: a refuted obligation or failed example |
| 3 | Checked program is incomplete or unverified |
| 2 | Usage error; usage text on stderr, no result |
| 4 | Source rejected, input unreadable, or solver unavailable; no proof standing |

In JSON mode a parsed check command always emits one result document on stdout,
including pre-check errors. Those errors have null standing and no definitions;
they are not failed proof attempts. In text mode pre-check errors go to stderr
with an ACC code. A crash, interrupted command, empty output, or unreadable JSON
must never be accepted as verified.

## JSON version 3

Object keys have no fixed order; arrays keep source order. This replaces receipt
version 2, which had no calls between functions; there is no legacy-format
switch. `accord diff` retains its separately
versioned [comparison format](diff.md).

| Field | Meaning |
| --- | --- |
| `receipt` | Schema version, now 3 |
| `accord` | Compiler package version |
| `file` | Input path as passed to check |
| `status` | checked, rejected, or unavailable |
| `standing` | verified, failed, incomplete, unverified; null before a check exists |
| `definitions[]` | Each checked function, in source order |
| `diagnostics[]` | Conditions requiring attention, pointing to canonical evidence |

Each definition has these fields:

| Field | Meaning |
| --- | --- |
| `name`, `kind` | Function name and function kind |
| `file`, `line` | Source file within a folder, or null for a single file; 1-based declaration line |
| `signature` | Function name, labeled parameters, and result type; no body |
| `parameters[]` | Parameter name and structured type |
| `result_type` | Structured result type |
| `types[]` | Used enums, each with name and source variants |
| `intent` | Unchecked declared purpose, or null |
| `specification[]` | Each source clause, with text and 1-based line |
| `hash` | SHA-256 of normalized checked semantics; names and intent do not affect it |
| `review_hash` | SHA-256 of this definition's review facts, excluding cache-use markers |
| `standing` | The function's standing: verified only when every function it calls, directly or not, is verified too |
| `unverified_callees[]` | Functions it calls, directly or not, that aren't verified; its own proofs assume their contracts |
| `obligations[]` | Every proof obligation with its outcome |
| `examples[]` | Every declared example with its execution result |
| `openness` | Open-case search result and all declared freedoms; null if no ensures |
| `strength` | Mutation result; null when not run |
| `not_proved[]` | Limits, each with a stable kind and explanatory message |

An obligation carries `kind` (postcondition, coverage, row_reachability,
overflow, precondition: a call meets its callee's `requires`, or bounds: an
index is within its list), `polarity`
(existential for row reachability, otherwise universal), `clause`, `line`,
`key`, `reused`, and `outcome`. Its key hashes the exact normalized SMT-LIB
script. Outcomes are proved, refuted, undecided, unsupported, or weak_callee.
Proved existential checks carry `witness`; refuted universal checks carry
`counterexample`; refuted existential checks carry `no_such_input: true`.
Undecided and unsupported outcomes carry a human explanation in `reason`.
A weak_callee outcome wasn't proved from the contracts of the functions it
calls, although the code may be right: its `counterexample` fails only when
each call in `calls[]` (the call as written, and the `value` the counterexample
gives it) takes a result its callee's contract allows and its body never gives.
Strengthening that contract can settle it; it makes standing incomplete, not
failed.
An agent routes undecided checks as unresolved regardless of that prose;
ACC0003 does not promise that a timeout retry will succeed.

Examples carry `text`, `line`, and `run` (passed or failed), with `reason` when
failed. Passing an example proves no universal property.

### Typed witness values

Inputs in witnesses, counterexamples, open cases, and mutants have a parameter
`name`, structured `type`, decoded `value`, exact `source` spelling, and `valid`.
Result values use the same shape without name. Types are:

- `{"kind":"bool"}`: value is a JSON Boolean.
- `{"kind":"integer","width":"i32"}` or i64: value is an exact decimal string,
  avoiding loss in clients with floating-point JSON numbers.
- `{"kind":"text"}`: value is a JSON string without Accord quotes.
- `{"kind":"enum","name":"Light"}`: value is a variant name; declared variants
  are in the definition's types inventory.
- `{"kind":"list","element":{"kind":"text"}}`: value is a JSON array of
  decoded element values, including an empty array.

Consumers can act on decoded data without parsing Accord prose or literals.
`source` supports reproducing a case; it is not the decoded value. If a manually
constructed report cannot be decoded, valid is false and value is null; never
accept that record as usable witness evidence. Normal compiler-produced witnesses
have been replayed by the interpreter before they are reported.

### Open cases and freedoms

`openness` is one of these shapes:

- `{"determined":true,"free":[...]}`: no ambiguity found outside freedoms.
- `{"open":{"inputs":[...],"results":[..., ...],"implementation":...},"free":[...]}`:
  one admitted input and two permitted results, plus the implementation's result.
- `{"undecided":"explanation","free":[...]}`: the search did not settle.

Every free entry is an exact `open when` clause. Open cases and undecided searches
make standing incomplete. No ensures means null, not determined. A domain enum
variant named Unknown is unrelated to an undecided solver outcome.

### Mutation and unproved claims

`strength` contains `tried`, `caught`, `equivalent`, `undecided`, `not_tried`, and
`survivors[]`. Each survivor has `change`, typed `inputs`, `original`, and `mutant`.
At most 64 mutants are tried per function. Equivalent mutants are proved to
behave the same on admitted inputs. A survivor changes behavior while meeting
the contract; it warns about contract strength but does not change standing.

Mutation requires ensures, enabled mutation analysis, and proved original
obligations. An invalid or unresolved baseline is not a basis for a contract-
strength claim. Null means not run, not a perfect score. Even no survivors means
only the finite reported mutation sample was checked.

`not_proved` always identifies intent alignment, surrounding-system integration,
and trusted compiler components as outside this proof. It also lists applicable
limits such as excluded inputs, absent examples, missing postconditions,
refuted/undecided/unsupported obligations, obligations blocked by a weak callee
contract, unverified callees, the assumptions about lists (their greatest
length and `count`'s axioms), failed examples, open cases, declared
freedoms, unrun or incomplete mutation analysis, and surviving mutants. These
are scope statements, not additional solver proofs or approvals.

## Stable diagnostics

Every diagnostic has `code`, `action`, `authority: compiler`, severity, source
location, and an `evidence` JSON pointer. Checked findings also name the function
and definition hash. Follow the pointer into the same result for the exact
clause, typed witness, counts, or reason; there is no second evidence ledger.
Pre-check errors instead have a message and null evidence; line is null when
there is no source location. Lines are 1-based; columns are not invented.

| Code | Condition | Action |
| --- | --- | --- |
| ACC0001 | Source rejected before obligations | repair_source |
| ACC0002 | Obligation refuted | inspect_counterexample |
| ACC0003 | Obligation undecided, including disagreement or rejected replay | inspect_solver |
| ACC0004 | Obligation unsupported | report_unsupported |
| ACC0005 | Example failed | inspect_example |
| ACC0006 | Undeclared open case | review_specification |
| ACC0007 | Non-equivalent mutant survives | review_specification |
| ACC0008 | Mutants undecided or untried | inspect_strength_limits |
| ACC0009 | Required solver unavailable | repair_environment |
| ACC0010 | Open-case search undecided | inspect_solver |
| ACC0011 | Input cannot be read | repair_environment |
| ACC0012 | No checked evidence for a function or program | add_checks |
| ACC0013 | Obligation not proved because a callee's contract allows results its body never gives | strengthen_callee_contract |
| ACC0014 | A function calls one that isn't verified | verify_callee |

Numbers identify conditions, never source lines, hashes, or particular solvers.
They are never reassigned. Prose may change. ACC0004 supports the public Outcome
state; the current source checker rejects unsupported constructs before producing
such an obligation. Its projection is tested, not advertised as a supported
language construct. Surviving-mutant and incomplete-mutation findings are warnings;
other findings require attention but do not redefine proof standing as failed.

A routing hint is not authority to weaken a specification, accept a freedom,
approve intent, or retry until a proof passes. Agents must require status checked
and standing verified, inspect warnings and limits, and reject unknown schema
versions or unknown codes before automatic acceptance. Empty diagnostics alone
is not proof of anything. Human acceptance remains a separate decision.

## Cache and freshness

The ledger in `.accord/` caches only proved, refuted and weak_callee outcomes,
keyed by the normalized script and compiler/solver builds. A script holds a
callee's contract, never its body; an outcome resting on a replayed model, which
runs the body, is also keyed by every callee's body. An unreadable or nonmatching entry
is recomputed; undecided checks run again. Examples always execute. Deleting
the ledger costs time only.

Changing intent updates the report without invalidating proof evidence, since
prose has no checked semantics. A saved review needs a separate identity:
review_hash binds current intent, signature, enum inventory, clauses, locations,
examples, outcomes, witnesses, strength, and limits. Cache-use markers are
excluded so an unchanged review does not become different merely through reuse.
Compare the current review_hash before relying on an older packet; it records
no approval or acceptance and cannot validate a packet against source by itself.
Solver versions are not embedded in this receipt; the compiler components remain
trusted, and model opinions and assumption approvals are not stored.
