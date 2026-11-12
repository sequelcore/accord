# Roadmap

Each stage runs end to end, from source to a result, and ends on a check
anyone can rerun. The targets that grow the language are real programs that
are already proved or tested elsewhere, and each stage adds only what its
target needs. The stages lead to a general-purpose language: verified rules
first, then state machines, data, complete programs with effects, a standard
library and native code. Tesota's proved rules were the first source; from
stage 7 each stage also takes a target from outside Tesota, so the language
isn't shaped by one product's needs. Every stage's target is chosen here in
advance; issues exist for the current stage and the next one, and later
stages will change as the earlier ones teach us.

## Decisions

Taken on 2026-11-12:

- **Every stage's target is planned in advance**, up to a general-purpose
  language. Each stage still builds only what its target needs, but the
  targets, from stage 10 to stage 15, are chosen now, so the path to whole
  programs and native code is explicit rather than decided one stage at a
  time. Each target is an outside project with its own tests and a license
  that allows porting, checked on 2026-11-12; how much of each test suite
  can run against the port is checked in that stage's issue, before its
  code. The public phase moves after stage 12, the first stage at which
  Accord writes complete programs.

- **Stage 9's semantics, before its code** ([#69](https://github.com/sequelcore/accord/issues/69)).
  A call is proved through the callee's contract, and a caller's proofs bind
  to that contract's hash. Lists are traversed by `all`, `any` and `count`,
  not by loops, so no `decreases` is needed; `count`'s properties are
  trusted axioms. Lists are immutable values. The encoding and solver
  options follow a [measurement](docs/design/measurements/2026-11-12-list-encoding/README.md).
  The [native code constraints](docs/design/native-code-constraints.md#decide-these-before-the-stage-that-needs-them)
  record each decision. The rule from outside Tesota is Cedar's evaluation
  of a policy set: denied when a `forbid` applies, allowed when a `permit`
  does.

- **Stage 7's readability check moves to the public phase.** A reader who
  wrote or maintains Accord can't show that its review is readable without
  the code, and no outside reader is available yet. Stage 7 closes with its
  view built and its done-criterion unmet; the check runs first in the public
  phase, before E2, on the packets as they stand then.

Taken on 2026-11-11:

- **Stages ordered by dependency.** `verdict-rule` needs calls between
  functions, quantifiers over lists, length, indexing and loops, and no state
  machines. Those constructs settle how calls are proved, value semantics
  and termination, which are costly to reverse, so they come before state
  machines. Work that needs people outside the project moves to a public
  phase after the stages below.

Taken on 2026-11-08:

- **Division truncates toward zero**, in code and in specifications alike:
  `-7 / 2 == -3` and `-7 % 2 == -1`, as in Rust, Java and C. A zero divisor
  and the one quotient that overflows (`MIN / -1`) are obligations. `/` and
  `%` are compiled when a target needs them.
- **General purpose, grown from the inside out.** Accord aims to be a
  general-purpose language. It starts with verified rules that run inside
  other programs and grows towards whole programs. Effects, a standard
  library and a native backend become stages of their own once a target
  needs them.

Taken on 2026-11-07:

- **Solvers through SMT-LIB 2.** The compiler writes each obligation as an
  SMT-LIB file and runs the solver as a separate program. Z3 and cvc5 must
  agree before an obligation counts as proved, and every obligation file can
  be rerun by hand. Bundling the solvers into releases comes later.
- **Integers:** fixed width (`i32`, `i64`) in code that runs, with proved
  absence of overflow; mathematical integers only inside specifications.
- **Syntax designed before the first proof**, not borrowed as a placeholder.
  It must read well to someone who has never seen Accord.
- **Rust** for the compiler. **MIT OR Apache-2.0.**

## Stages

| Stage | Delivers | Done when | Status |
| --- | --- | --- | --- |
| 0. Foundation | Workspace, pinned toolchain, CI on three platforms, `accord --version` | CI passes on Windows, Linux and macOS | Done |
| 1. Language design | The surface syntax and the core concepts (obligation, evidence, intent, open case, staleness), each with one meaning, written as a reference with examples | Comprehension check passes (below) | Done: the models arm of the comprehension check passed (version 2) |
| 2. First proof | Pure, total functions over `Bool`, fixed-width integers and enums; `requires` and `ensures`; obligation generation to SMT-LIB; counterexamples in source terms; an interpreter for `example`s; the result type and the conformance harness | Tesota's `proof-outcome-rule` ported and proved; every planted fault refuted, with a counterexample or, for a row no input reaches, a proof that none does; a solver timeout reported as undecided | Done ([#10](https://github.com/sequelcore/accord/issues/10)): `conformance/cases/proof-outcome.accord` and its seven planted faults |
| 3. Evidence bound to content | A hash per normalized definition, an evidence ledger by hash, the receipt as the compiler's output (`--json` and text), incremental checking | Changing a body makes exactly its own and its dependents' evidence stale; a rename changes none | Done ([#11](https://github.com/sequelcore/accord/issues/11)): `crates/accord/tests/ledger.rs` |
| 4. Contract strength | Mutation of function bodies; proved weakening between two revisions of a specification (`accord diff`) | Planted weakenings detected, strengthenings not flagged; `weakening-rule` and `mutation-rule` ported | Done ([#26](https://github.com/sequelcore/accord/issues/26)): `conformance/cases/weakening.accord`, `mutation.accord` and their faults; `crates/accord/tests/diff.rs` |
| 5. Open cases | For each specification, a search for one input with two permitted outputs; `open` declarations; no verified result while an undecided `open` applies | Experiment E1 registered and run | Done ([#27](https://github.com/sequelcore/accord/issues/27)): E1 run; [result](docs/design/e1-open-cases/README.md#result-of-the-run-2026-11-09) |
| 6. Data | `Text`, `List<T>` with `in` (no quantifiers), and a folder as one program: what `gate-rule` needs | `gate-rule` ported and proved | Done ([#33](https://github.com/sequelcore/accord/issues/33)): `conformance/programs/tesota/`, with text, lists and a folder as one program; records, sum types and `match` moved to the stage that needs them |
| 7. Intents and the review view | `intent` declarations; the human view (intent, specification, examples, counterexamples, open cases, surviving mutants, what wasn't proved); the agent view as structured diagnostics | The review of `gate-rule` and an outside-domain rule is readable without their code | Built, check moved to the public phase ([#46](https://github.com/sequelcore/accord/issues/46)): the [gate and Cedar packets](docs/design/stage-7-review/README.md) await a reader from outside the project |
| 8. TypeScript output | Accord compiled to a TypeScript module | Tesota's own tests pass with the generated rule in place of the original | Done ([#47](https://github.com/sequelcore/accord/issues/47)): `accord build` ([reference](docs/typescript.md)), checked against the interpreter; Tesota's typecheck and tests pass with the generated rules (`scripts/check-tesota.sh`), and axios-retry's lint and specs with its generated request predicates (`scripts/check-axios-retry.sh`) |
| 9. Whole rules | Calls between functions, proved through the callee's contract; `all`, `any` and `count` over lists; length and indexing: what `verdict-rule`, the largest rule, and Cedar's policy-set evaluation need | `verdict-rule` and Cedar's policy-set evaluation ported and proved | In progress ([#69](https://github.com/sequelcore/accord/issues/69)): calls between functions, proved through the callee's contract (`conformance/cases/call*.accord`) |
| 10. State machines | `machine` declarations, invariants, explicit-state model checking, planted faults | A Tesota design model ported, its faults detected; the circuit breaker of [opossum](https://github.com/nodeshift/opossum) (Apache-2.0), closed, open and half-open, proved, and opossum's tests pass with the generated module | |
| 11. Structured data | Records, sum types and `match`, recursion with `decreases`, `Text` operations beyond equality | Version comparison from [node-semver](https://github.com/npm/node-semver) (ISC) proved, and node-semver's tests for it pass with the generated module | |
| 12. Complete programs | Effects (files, standard input and output, exit codes) declared in types, errors as values, a `main` | `wc` from [uutils/coreutils](https://github.com/uutils/coreutils) (MIT) written only in Accord, its effects in its types and its proofs still fail-closed; uutils' tests for `wc` pass against it | |
| 13. Standard library and modules | Maps and sets, generic types, imports between packages, as stages 11 and 12's programs need them | Accord's own E2 tooling (`docs/design/e2-human-review/materials.py` and `score.py`) rewritten in Accord; their self-tests and CI check pass | |
| 14. Native code | A native backend (LLVM), and validation of its output before it relies on a proof | Stage 12's `wc` compiled natively; uutils' tests pass, and a published measurement against uutils' Rust `wc`, with the same algorithm and each side's safety checks stated | |
| 15. Tools | A language server, a formatter, packages | Stages 11 to 14's programs maintained with them, without knowledge of the compiler | |

**Public phase**, after stage 12: what needs people outside the project.
Stage 7's readability check runs first, then E2 and E4 run, a first public
alpha is published, and the project opens a place for public discussion.
Stages 13 to 15 follow in public. Agent experiments (E3's full run, E1 on
larger contracts) run when capacity allows, each recording the compiler
revision it used.

Later, each when a stage needs it: escape routes to Lean and Dafny for what
SMT can't decide, and translation validation of the compiler's own output.

## Experiments

Each is registered, with its measure and success rule, before any run.

| Experiment | Question | Runs at |
| --- | --- | --- |
| [Comprehension check](docs/design/comprehension-check/README.md) | Do people and models who have never seen Accord explain its snippets correctly without a reference? | Stage 1 |
| [E1. Mechanical open cases](docs/design/e1-open-cases/README.md) | Does the solver find the ambiguities that asking an agent finds, or more? | Stage 5 |
| [E2. Human review](docs/design/e2-human-review/README.md) | Do reviewers find more intent errors per minute with the review view than with code or specifications alone? | Public phase; draft, not registered or run |
| [E3. Agents writing Accord](docs/design/e3-agents-writing/README.md) | Is verified success with Accord comparable to TypeScript with LemmaScript on the same rules? | Registered; runs when capacity allows, on the subset it was registered for |
| [E4. Fixing a failed proof](docs/design/fix-experiment/README.md) | Can a developer who has never seen Accord turn a failed check into a verified fix that keeps the intent? | First public alpha; again on a later public version with the review view (stage 7) |

## Trust

Each release states what it trusts: the parser, the obligation generator, the
solvers and the interpreter. Nothing about the compiler itself is proved
until translation validation exists.
