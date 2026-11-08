# Influences

What Accord takes from other languages, and what it leaves, for the stage 1
syntax design. Each borrowing must serve one of the design goals below; a
feature that is merely good elsewhere doesn't qualify.

## Design goals

1. **Readable by someone who has never seen Accord.** A reader who knows any
   mainstream language explains a snippet correctly without a reference.
   The stage 1 comprehension check tests this goal.
2. **Specifications read as sentences.** A person judges intent from the
   specification, examples and counterexamples, without opening the body.
3. **Familiar to agents.** Prefer forms that agents have seen often, so the
   cold start costs as little as possible.
4. **One obvious way.** A small surface and one canonical formatting, so two
   authors write the same thing the same way.
5. **Nothing hidden.** No implicit conversions, no exceptions, no null, no
   hidden control flow; everything the compiler must prove is visible in the
   source.

## Taken

| From | What | Goal it serves |
| --- | --- | --- |
| Dafny | The verification keywords: `requires`, `ensures`, `invariant`, `decreases`, `old(...)`, `==>`, `<==>` | 2, 3: the best-known verification vocabulary, and Dafny's is the one agents verify best with |
| Verus | Fixed-width integers in code that runs, mathematical integers in specifications | 5: the integer rule in the [roadmap](../../ROADMAP.md#decisions), already proven workable |
| Rust | Expression-oriented blocks, exhaustive `match`, enums that carry data, explicit integer widths, errors as values | 5: every case handled, nothing thrown |
| TypeScript | `name: Type` annotations, braces, string-like enum names, `const` | 1, 3: the most widely read surface for both people and agents |
| Gleam | A small language with one way to do things, labelled arguments, `pub` for exports | 1, 4: labelled arguments make calls with several booleans readable |
| Swift | Argument labels read at the call site; readable keywords over symbols | 1 |
| Elm | Diagnostics that teach: what is wrong, where, and what to do | 1: the compiler is the language's first reader-facing surface |
| Go | One canonical formatter from the first release | 4 |
| Unison | Definitions identified by the hash of their meaning, names as labels | Evidence bound to content ([concepts](../concepts.md#evidence)) |
| Bend 2 | Human-owned laws kept apart from agent-written code | Approval ([concepts](../concepts.md#approval)) |
| Lean | Proofs as part of the language, `#eval`-style examples beside definitions | Examples as stated intent; Lean as an escape route |
| Haskell guards, DMN decision tables | Ordered rows of a condition and a result, where the first match decides (`decide`) | 2, 4: a decision rule written once, as body and specification together ([probe](probes/2026-11-07-intent-and-decision-tables/README.md)) |
| Koka | Effects in function types | Deferred until a stage needs effects |

## Left

| From | What | Why |
| --- | --- | --- |
| Lean, Bend 2 | Dependent types and hand-written proof terms in the core surface | Goal 3: interactive proofs are where agents verify worst; they stay an escape route |
| Rust | Lifetimes and borrow annotations | Goal 1: no references in the early subset, so nothing to annotate |
| TypeScript | `number`, `any`, implicit coercion | Goal 5: nothing provable about them |
| Dafny | The `method` and `function` split, `ghost` noise | Goal 4: one kind of function; specification context decides integer meaning |
| Python, Bend 2 | Significant indentation | Goal 4: braces survive copy, paste and generation without changing meaning |
| Unison | Editing a codebase stored in a database | Files stay the source; identity by hash happens underneath |
| Macro systems | `macro_rules`, templates | Goal 5: hidden code the reader can't see |

blood, announced on 2026-11-07, has published nothing concrete yet; it is
left out of this table until it does.
