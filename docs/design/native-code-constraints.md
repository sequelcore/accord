# Native code constraints

**Status:** approved by the maintainers on 2026-11-11. Nothing on this page
starts a backend or changes the [roadmap](../../ROADMAP.md), which still owns
the stages and their order. This page owns the semantic constraints that keep
native code possible: what the compiler already guarantees, what a backend may
do with those guarantees, and which decisions are costly to reverse.

A backend means any code generator: stage 8's TypeScript output first, a
native backend later. Terms are defined in the [concepts](../concepts.md).

## Decisions the compiler already enforces

| Area | Decision | Source | What a backend may do with it |
| --- | --- | --- | --- |
| Integers | `i32` and `i64` in code that runs. Overflow is an obligation, so a verified definition never overflows | [syntax](syntax.md), `obligation.rs` | Emit unchecked arithmetic, under the [conditions](#use-a-proved-fact-only-under-six-conditions) below |
| Integers | Mathematical integers in `requires` and `ensures` only | [roadmap](../../ROADMAP.md#decisions) | Erase specifications. They never run |
| Evaluation | An overflow obligation holds only where its operation runs: the branch of an `if` that runs, the row that decides | [syntax](syntax.md) | Evaluate a branch that doesn't run only with arithmetic that can't trap and relies on no proof, since its obligation doesn't hold there |
| Evaluation | Both operands of `and` and `or` must stay in range, whatever the other's value | [syntax](syntax.md) | Choose short-circuit or eager evaluation |
| Functions | Pure and total. No exceptions, null or hidden control flow | [influences](influences.md) goal 5 | Reorder, share or skip repeated work |
| Decision tables | The first row whose condition holds decides. Coverage and reachability are proved | [syntax](syntax.md) | Drop the path where no row matches |
| Identity | A hash identifies a definition's normalized representation | `identity.rs` | Reuse output on a hash match ([below](#a-hash-identifies-a-representation-not-a-meaning)) |
| Trust | Two solvers must agree. The interpreter replays every model. The receipt lists the trusted components | `solver.rs`, [receipt](../receipt.md) | Use the interpreter as the reference for generated code |

## Decisions approved but not implemented

- **Division** truncates toward zero, and a zero divisor and `MIN / -1` are
  obligations ([roadmap](../../ROADMAP.md#decisions)). The tokenizer still
  rejects `/` and `%`.
- **Lists in stage 9** ([#69](https://github.com/sequelcore/accord/issues/69)):
  a list is a sequence in SMT-LIB. `all`, `any`, `count`, `length` and
  indexing read it by index. `in` stays `seq.contains`, or a disjunction
  over a literal list, and a script that also reads lists by index states
  how each `in` relates to the indices, since neither solver connects the
  two ([measurement](measurements/2026-11-12-list-encoding/README.md)).
  Encoding every `in` by index, as first decided, made Z3 give up on
  inputs for Tesota's gate rule, whose `not failure in sent` became a
  `forall`. cvc5 runs with `--mbqi`, without which it refutes no quantified
  claim. `count`'s bounds, its zero case and its full case are axioms the
  compiler states, and every list holds at most 4,294,967,295 elements;
  the receipt lists both as trusted.
- **Machines in stage 10** ([#82](https://github.com/sequelcore/accord/issues/82)):
  an invariant is proved only when Z3 and cvc5 prove it inductive, from the
  initial states and through every event. An invariant is refuted only by a
  trace from an initial state, found by the explorer and replayed by the
  interpreter; a planted fault is detected only that way. An obligation the
  solvers refute without such a trace is `not_inductive`, with the state
  they found, much as a contract too weak to prove a caller is
  `weak_callee`. What the explorer covers without finding a trace is
  recorded on the unproved obligation, never as an outcome, and is trusted,
  not proved.
  `accord build` writes a machine only when its invariants are proved
  ([measurement](measurements/2026-11-12-stage-10-machines/README.md)).
- **No new surface syntax** without evidence that the current one limits
  comprehension or productivity.
- **A structured interface for agents** comes after
  [E3](e3-agents-writing/README.md), built on definition hashes, the receipt
  and `accord diff`.
- **Several verified implementations of one specification** are a research
  question. No syntax for it is adopted; stage 11's `spec fn` is a
  specification function, not a specification apart from an implementation.
- **Evidence for decisions:** a major decision cites local evidence (a
  registered experiment, a conformance case or a measurement in this
  repository) or a primary source read at the source.

## Use a proved fact only under six conditions

A proved fact may change generated code only when all six conditions hold.
When any fails, the backend emits the checked or general form, so the result
stays fail-closed.

1. **Verified standing.** The definition is verified: both solvers proved
   every obligation, none is undecided or unsupported, and its evidence is
   current.
2. **Same representation.** The evidence's hash matches the normalized
   representation being compiled, with the same solver versions.
3. **Exact fact.** The fact is one proved obligation for that operation under
   its guards: the overflow obligation of this `+`, at this width, in this
   branch. A fact proved elsewhere or under other guards doesn't transfer.
4. **Admitted inputs.** The fact assumes the function's `requires`. Every
   caller is verified code whose call obligations are proved, or enters
   through a [boundary](#check-at-the-boundary-prove-inside-the-core) that
   checks the `requires`.
5. **Matching meaning.** The target instruction means what Accord's operation
   means at that site. In LLVM, `nsw` on `add`, `sub` and `mul` of the
   declared width means no signed wrap, which is the overflow obligation.
   `sdiv` needs both the zero-divisor and the `MIN / -1` obligations.
6. **Stated trust.** In LLVM, a wrong `nsw` produces poison, which can become
   undefined behavior. A proof error then becomes undefined behavior rather
   than a wrong value. The receipt lists the solvers, the obligation generator
   and the backend as trusted for every flag that relies on a proof.

The same conditions govern every other flag that relies on a proof, such as
range metadata and an unreachable default in a compiled decision table.

## Check at the boundary, prove inside the core

The **verified core** is code whose standing is verified and whose callers are
verified too. A **boundary** is every other way in: a call from TypeScript,
input parsed from outside, a host value, or an unverified caller.

- At a boundary, generated code checks explicitly what the core assumes: each
  integer parameter's range, each enum's value and the function's `requires`.
  A failed check rejects the call with an error and never runs the core.
- A boundary check doesn't weaken the core. Inside, proofs still remove
  checks. The boundary check is what makes condition 4 hold for callers that
  aren't verified.
- The receipt lists each boundary and its checks, so a reader sees where proof
  ends and checking begins.
- Stage 8's exported functions are boundaries, since their TypeScript callers
  aren't verified.

## Test generated code now, prove it later

Differential testing and formal translation validation give different
assurance. The receipt names which one applies.

| | Differential testing | Formal translation validation |
| --- | --- | --- |
| Claim | The generated code and the interpreter agree on the inputs tried | The generated code refines the source semantics for every input, for this compilation |
| Method | Run both and compare | Prove each output equivalent to its source, by SMT or in a proof assistant |
| Outcome reported | Tested | Proved |
| Finds a compiler bug on an input not tried | No | Yes, within the validator's model |

Stage 8's [TypeScript output](../typescript.md#what-it-trusts) uses
differential testing, in Accord's own test suite:

- **Exhaustive** where the domain is finite: every combination of `Bool` and
  enum parameters, up to 4,096 combinations per function. Past that, a seeded
  sample of them is taken.
- **Selected** for integers, `Text` and lists: each width's boundaries (`MIN`,
  `MIN + 1`, -1, 0, 1, `MAX - 1`, `MAX`; for `i64`, the safe integers'), a
  seeded sample, the program's text literals and characters beyond ASCII,
  every input a solver gave and every `example`.
- **Stated limits:** each module's header says it isn't proved equivalent,
  and the TypeScript reference states which inputs the tests use. The receipt
  doesn't report on generated code yet.

Formal translation validation is required before a backend uses any flag
that relies on a proof (condition 6).

## A hash identifies a representation, not a meaning

`identity.rs` renames a function's parameters to `p0`, `p1`, …, the enums it
uses to `E0`, `E1`, … and their variants by position, then hashes the result.

- Equal hashes mean equal normalized representations. They mean equal
  behavior only as far as normalization is correct, and normalization is
  trusted, not proved.
- Different hashes say nothing about behavior. `a + b` and `b + a` hash
  differently and always give the same result.
- Equivalence is a separate claim that a solver proves: `accord diff` for
  specifications, an equivalent mutant for bodies. A hash never implies it.
- A change to the normalization changes every hash, so it is a costly
  decision (below).

## Decide these before the stage that needs them

Reversing any of these invalidates stored evidence, generated code or a
public interface. Each is settled by a recorded decision, with evidence,
before the stage that first depends on it. Rows are in dependency order.

| # | Decision | State | Depends on | Settle before | Cost of reversing |
| --- | --- | --- | --- | --- | --- |
| 1 | Integer semantics | Enforced | — | — | Every proof and all stored evidence |
| 2 | Purity as the default; the semantics of effects | Purity enforced; effects undecided. A machine needs none: its events are pure functions of the state and their parameters, and time is a field the caller advances (stage 10, approved) | 1 | The first target that needs input or output | Freedom of evaluation order, reordering, proofs of calls |
| 3 | Freedom of evaluation order for `and` and `or` | Enforced | 1, 2 | Effects | Generated code that evaluates eagerly |
| 4 | Totality, and how termination is proved | Decided: no recursion and no general loop. A list is traversed only by `all`, `any` and `count`, which end on every finite list, so termination holds without `decreases`. Recursion is rejected (`conformance/cases/call-recursive.accord`, `call-mutual.accord`). Stage 11 (approved, not implemented, [#83](https://github.com/sequelcore/accord/issues/83)): a function may call itself with `decreases`, an integer measure proved smaller and non-negative at each call; mutual recursion stays rejected | 2 | Recursion or loops | What "proved" means for code that may not return |
| 5 | Value semantics: no aliasing a program can observe | Decided: a list is an immutable value, `==` compares contents, and no operation updates a list or exposes its identity. A machine's state is an immutable value of its fields, and an event returns the next state (stage 10, approved). A record or a variant with fields is an immutable value compared field by field, with no update syntax (stage 11, approved) | 2 | Records, or updates to lists | Every memory strategy at once |
| 6 | The encoding of `Text` | Decided for equality: a sequence of characters up to U+2FFFF, the solvers' alphabet, which the [boundary](../typescript.md#the-boundary) enforces. Length and `<` count and compare code points, as the solvers do; generated code uses its own helper, not JavaScript's UTF-16 `.length` and `<` (stage 11, approved, not implemented) | 5 | The first `Text` operation beyond equality, and stage 8's boundary, since TypeScript strings are UTF-16 | Every `Text` proof and boundary check |
| 7 | Proving calls: callers rely on the callee's contract, not its body | Enforced: at each call, the callee's `requires` is an obligation and its `ensures` an assumption; recursion is rejected (`conformance/cases/call*.accord`) | 1, 2, 4 | Calls between functions | The cost of proofs, separate compilation, inlining |
| 8 | Normalization and hashing | Enforced. A machine's fields and events are normalized by position, as parameters and variants are (stage 10, approved) | 1 | Any change to the normal form | Every stored hash and piece of evidence |
| 9 | An identity for a specification apart from its implementation | Partly enforced: a caller's proofs bind to a hash of the callee's contract, so a change to the callee's body alone leaves them current (`identity.rs` tests). How approval binds is undecided | 7, 8 | Several implementations of one specification | How approval and evidence bind |
| 10 | The boundary contract: what is checked and which error a failed check returns | Decided: [types, ranges and each `requires`](../typescript.md#the-boundary), with `TypeError` or `RangeError`. A machine's state crosses as an object of its fields in `camelCase`, checked for types, ranges and every invariant, helper invariants included; each event is an exported function from the state and its parameters to the next state, and each observer one from the state (stage 10, approved) | 1, 5, 6 | Stage 8's first published output | A public interface, once others call it |

## Keep these optimizations possible

A **permitted** optimization can't change a verified result, because the
semantics allow it. Whether it **pays off** is a separate question that only a
measurement on a real workload answers. None of the payoffs below is measured.

| Optimization | Permitted because | Expected payoff, unmeasured | Rule that keeps it possible |
| --- | --- | --- | --- |
| No checks at run time for overflow, division or bounds | Each check is a proved obligation | Small, mostly in tight loops where checks block vectorization. Code with no checks, as C has, gains nothing | Never replace a proof with a check at run time inside the core |
| Integer ranges passed to the optimizer | `requires` and proved ranges | Small and local | Specifications never affect behavior |
| Reordering, shared subexpressions, memoization, parallel execution | Purity and totality | Modest for reordering. Memoization and parallelism can also slow code down | Effects appear in types, and pure is the default |
| Any memory strategy: copying, reference counting with reuse in place, regions, the stack | Value semantics | Possibly large for code that allocates often; unknown | No operation lets a program observe sharing |
| Decision tables as jump tables or code without branches | Proved coverage and reachability, first row decides | Small: LLVM already compiles `switch` well | `decide` stays a closed construct with proved rows |
| Data layout, such as a structure of arrays | No pointers or addresses in the language | Possibly large for loops over much data | The language exposes no addresses or layout |
| Several implementations of one specification, the fastest kept | Contracts are checked apart from the body | Possibly the largest, at the highest cost in proofs and benchmarks | Specifications stay separately identifiable |
| Output reused on a hash match | Equal hashes mean equal representations | Compile time only | The hash covers the normalized representation, not the text |

## Change this page

A row moves from undecided to decided only through a recorded decision that
meets the evidence rule above. A decision counts as enforced only when
conformance cases cover it. A performance claim needs a measurement with the
same algorithm on both sides and the safety checks of each stated.
