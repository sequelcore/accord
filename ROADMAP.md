# Roadmap

Each stage runs end to end, from source to a result, and ends on a check
anyone can rerun. The targets that grow the language are real rules that are
already proved or tested elsewhere, smallest first: each stage adds only what
its next rule needs. Tesota's proved rules were the first source; from stage 7
each stage also takes a rule from outside Tesota, so the language isn't
shaped by one product's needs. Issues exist
for the current stage and the next one; later stages are planned here and will
change as the earlier ones teach us.

## Decisions

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
| 7. Intents and the review view | `intent` declarations; the human view (intent, specification, examples, counterexamples, open cases, surviving mutants, what wasn't proved); the agent view as structured diagnostics | The review of `gate-rule` and an outside-domain rule is readable without their code | In progress ([#46](https://github.com/sequelcore/accord/issues/46)): [gate and Cedar packets](docs/design/stage-7-review/README.md) await human review; E2 remains draft |
| 8. TypeScript output | Accord compiled to a TypeScript module | Tesota's own tests pass with the generated rule in place of the original | In progress ([#47](https://github.com/sequelcore/accord/issues/47)): `accord build`, checked against the interpreter ([reference](docs/typescript.md)); Tesota's typecheck and tests pass with the generated rules (`scripts/check-tesota.sh`); the outside rule remains |
| 9. State machines | `machine` declarations, invariants, explicit-state model checking, planted faults | A Tesota design model ported, its faults detected | |
| 10. `verdict-rule` | What the largest rule needs | Ported and proved | |

Later, each when a stage needs it: escape routes to Lean and Dafny, an LLVM
backend, effects, a language server, translation validation of the compiler's
own output.

## Experiments

Each is registered, with its measure and success rule, before any run.

| Experiment | Question | Runs at |
| --- | --- | --- |
| [Comprehension check](docs/design/comprehension-check/README.md) | Do people and models who have never seen Accord explain its snippets correctly without a reference? | Stage 1 |
| [E1. Mechanical open cases](docs/design/e1-open-cases/README.md) | Does the solver find the ambiguities that asking an agent finds, or more? | Stage 5 |
| [E2. Human review](docs/design/e2-human-review/README.md) | Do reviewers find more intent errors per minute with the review view than with code or specifications alone? | Stage 7; draft, not registered or run |
| [E3. Agents writing Accord](docs/design/e3-agents-writing/README.md) | Is verified success with Accord comparable to TypeScript with LemmaScript on the same rules? | Stage 7; registered, not run |
| [E4. Fixing a failed proof](docs/design/fix-experiment/README.md) | Can a developer who has never seen Accord turn a failed check into a verified fix that keeps the intent? | First public alpha; again on a later public version with the review view (stage 7) |

## Trust

Each release states what it trusts: the parser, the obligation generator, the
solvers and the interpreter. Nothing about the compiler itself is proved
until translation validation exists.
