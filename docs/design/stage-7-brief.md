# Stage 7: intents and the review view

Design brief for the owner. No implementation or experiment is authorized by
this proposal. Research cutoff: 2026-11-09. Accord baseline:
`d4a7fc8a96957103b98e89731d5e2e38a192613d`.

## Findings first

**Verified** means the cited source was opened and read, including local source
checkouts. It does not mean its claims were independently replicated.
**Reported** means secondhand and not verified. Recommendations and proposed
thresholds below are design judgments, not research findings.

1. **Verified: prose alone does not solve specification review.**
   [Kaufman et al., §§3–7](https://arxiv.org/html/2607.08885v1) studied 86
   Python programmers, with a ten-person think-aloud follow-up. Judgment accuracy
   was 49% for incorrect postconditions and 74% for correct ones. Comments gave
   no overall accuracy benefit; poor comments could hurt accuracy while raising
   confidence. Participants assessed assertions against implementations assumed
   correct, rather than reviewing Accord proofs against independent requirements.
   Negative examples appeared in their reasoning, but this was not a randomized
   test of displaying counterexamples. This motivates E2; it does not establish
   that Accord's proposed view closes the gap.
2. **Verified: selected examples have controlled evidence behind them.**
   [Gerdes et al., 2018, §§5–6](https://research.chalmers.se/publication/510284/file/510284_Fulltext.pdf)
   randomly divided 22 Erlang students between heuristic-generated and random
   example suites. They predicted results of banking API calls without seeing
   the model. The selected examples improved understanding. This tests example
   selection, not proof review or defect detection. Prefer contrasting cases
   that expose a requirement over a long list of routine successes.
3. **Verified: explanations grounded in counterexamples can help, with limits.**
   [Kaleeswaran et al., 2023, §§5–8](https://link.springer.com/article/10.1007/s10664-023-10353-4)
   surveyed 41 Bosch engineers and conducted a 13-person one-group pretest–posttest
   study. The latter compared raw model-checker output with explanations linking
   violations, specifications, components, and erroneous state variables. Complete
   identification of inconsistent specifications increased from four to nine
   participants. Different tasks and fixed order prevent clean causal attribution.
   This supports testing localized witnesses, not a promised effect size.
4. **Verified: showing coverage can save time without improving adequacy.**
   [Ivanković et al., 2024, §§2–4](https://homes.cs.washington.edu/~rjust/publ/productive_coverage_icse_2024.pdf)
   hid coverage information in 30,000 randomly selected Google code reviews.
   Displaying it reduced review duration; the experiment found no evidence of
   additional coverage improvement beyond review itself. Their later Productive
   Coverage intervention is separate. Faster review and better defect detection
   must be separate E2 measures.
5. **Verified: today's evidence is useful but narrower than a review view.**
   The [receipt](../receipt.md), [parser](../../crates/accord/src/syntax/parser.rs),
   and actual run below agree: `intent` is consumed and discarded; diagnostics
   have file, line, and prose but no code. The receipt has obligation clauses,
   outcomes and witnesses, executed examples, strength, and openness. It lacks
   retained intent, a complete specification inventory, and an explicit inventory
   of unproved claims. The Tesota folder is **incomplete**, although
   `gate_verdict` itself is **verified**. Reporting only its green result would
   hide the open case in another function.
6. **Verified: Cedar is a small, suitable second domain.** At
   [commit 5af7b8c](https://github.com/cedar-policy/cedar/commit/5af7b8cd78e4e0da3ae23f1715023fff8e62941d),
   `PartialResponse::decision` reduces four collection-emptiness questions to
   five ordered cases. Its Boolean abstraction passed today's Accord checker
   without new constructs. This proves the proposed abstraction, not Cedar's
   evaluator or a translation between the two implementations.
7. **Reported: the earlier target report recommends Cedar before Kubernetes or
   Go.** That report was not supplied or found in the required repository
   documents. Its ranking is not treated as evidence. The Cedar recommendation
   here rests on source inspection and the scratch check below.

Recommendation: retain intent as unchecked text and make the human view a
projection of the same evidence agents receive. Put limits beside successes.
Use examples, witnesses, and explicit scope to help reviewers challenge a
specification. No controlled study found in this search isolates an explicit
“not proved” list, or tests this entire bundle. Its benefit remains an E2 hypothesis.

## What comparable tools expose

The documentation column records **verified tool descriptions**, not measured
usability. The reports column records **verified firsthand reports**, not
reproduced defects or representative prevalence. Historical reports do not imply
that the problem remains in current releases.

| Tool | What its documentation exposes | What people report; consequence for Accord |
| --- | --- | --- |
| Dafny | The [reference, counterexamples and test generation](https://dafny.org/dafny/DafnyRef/DafnyRef#sec-counterexamples) distinguishes verification from generated tests and describes experimental counterexamples and coverage choices. | [Issue #5921](https://github.com/dafny-lang/dafny/issues/5921) shows a counterexample mentioning a variable before its declaration, with an invalidity warning. Keep source-level witness provenance and replay status visible. Never silently polish a questionable solver model into a fact. |
| Verus | Its [contract tutorial](https://verus-lang.github.io/verus/guide/requires_ensures.html) connects failed preconditions to call sites and failed guarantees to clauses. Its [overview](https://verus-lang.github.io/verus/guide/) excludes verification of Rust/LLVM and the verifier itself. | [Discussion #387](https://github.com/verus-lang/verus/discussions/387) collects misleading spans, generated-variable errors, and a zero-verified summary following a compiler error. Show both the failing clause and affected function, and distinguish rejection from an empty successful check. |
| Lean | The [reference](https://lean-lang.org/doc/reference/latest/Interacting-with-Lean/) exposes goals, messages, and `#print axioms`, including transitive dependencies. Completion of goals is different from an acceptable axiom set. | A [new-user report](https://leanprover-community.github.io/archive/stream/113489-new-members/topic/Very.20confusing.20error.20message.html) describes dependency errors mixed into the current file's InfoView. Keep each finding's source and dependency scope explicit. |
| F* | The [SMT tutorial](https://fstar-lang.org/tutorial/book/part1/part1_prop_assertions.html) shows refinement propositions, assertion failures, and solver heuristics. The quantified domain changes the proposition's meaning. | [Pulse #477](https://github.com/FStarLang/pulse/issues/477), in the F* ecosystem rather than the base verifier, reports an opaque recursion error absent from the editor, with a maintainer linking a crash fix. No direct base-F* user study was verified in this search. Preserve diagnostics across terminal and agent views. |
| Kani | [Results](https://model-checking.github.io/kani/verification-results.html) list per-property status, description, and location, including unreachable and undetermined checks. The [unwinding tutorial](https://model-checking.github.io/kani/tutorial-loop-unwinding.html) explains harness limits and insufficient unwinding. | [Issue #3910](https://github.com/model-checking/kani/issues/3910) reports confusion when loop and function contracts did not compose in version 0.59; it links a closing change. List the exact admitted domain and checked properties instead of a general correctness badge. |
| Liquid Haskell | [Documentation](https://ucsd-progsys.github.io/liquidhaskell/) presents refinement contracts, totality, bounds, and termination; the [specification reference](https://ucsd-progsys.github.io/liquidhaskell/specifications/) separates specifications for dependencies. | [Issue #2745](https://github.com/ucsd-progsys/liquidhaskell/issues/2745) reports missing class constraints surfacing as unbound dictionary symbols. Name the source condition and actionable cause, not solver-internal names. |
| TLA+ / Apalache | [TLC](https://tla.msr-inria.inria.fr/tlatoolbox/doc/model/executing-tlc.html) separates errors from trace states. [Apalache](https://apalache-mc.org/docs/apalache/running.html) documents bounded checking through `--length` and separate simulation. Its [ITF design](https://apalache-mc.org/docs/adr/015adr-trace.html) gives traces a structured format. | ITF's authors report that TLA+ trace notation looks alien to engineers unfamiliar with it. This is a tool-author observation, not an independent user study. [Issue #3385](https://github.com/apalache-mc/apalache/issues/3385) reports silent folding over zero elements for unsupported set forms. Keep unsupported separate from proved; display bounds where they exist. |
| Code-review tools | [GitHub](https://docs.github.com/en/pull-requests/reference/pull-request-reviews) separates line comments, proposed changes, and approval decisions. It does not claim approval is a proof. | The Google field experiment above shows that a useful display need not improve adequacy. Preserve human acceptance separately; measure defects and time separately. |

These are design inputs, not templates. A Lean goal explorer would overwhelm a
reviewer of a small decision rule. A raw trace alone hides why a case matters.
A code diff cannot explain the promises absent from that code. Accord can start
with a short function report because today's consumer is reviewing small pure
rules, not navigating interactive proof tactics or state machines.

## Five decisions for the owner

### 1. Start with terminal text, or require a page too?

Options: terminal text; a generated static page; both. Recommend **terminal text
first**, using `accord check <program>` and its existing JSON counterpart.
Make the default text a per-function review view rather than introducing a second
check path. The same structured record must drive both renderers. A page offers
folding and sharing, but adds layout, artifact freshness, and accessibility work
before its value is measured. Both doubles presentation verification.

Consequence: E2 initially tests the terminal presentation. If the readability
gate below fails because of navigation or density, reconsider a static page
before registration. Do not change presentation during E2 or claim its result
applies to an untested page. No browser service, account, or approval storage is
needed for stage 7.

### 2. What does “readable without its code” mean before E2?

Options: owner inspection; a model comprehension check; a small human gate.
Recommend **three human readers** who did not author these ports, including one
without formal-methods experience. Models may help find wording problems but
do not stand in for human readers.

Give each reader only the view and a one-page glossary, with no function bodies.
For each rule, ask eight questions: purpose; input domain; output meanings;
two contrast-case predictions; an excluded or unchecked claim; meaning of
determined versus correct intent; and whether the evidence proves the surrounding
system. Use the actual received view, not a manually improved study version.

Before showing it, freeze an answer key reviewed by the owner against the
upstream rule, Accord contract, and receipts. For gate cases include a repeated
failure and a timeout. For Cedar include both a satisfied permit with a residual
forbid and no possible permit with a residual forbid. Each reader must score at
least 7/8 **on each rule**, with no error on proof scope or `Unknown` versus
solver uncertainty. Record anonymous answers, scoring, artifact hashes, reader
experience, and failures. Revise and repeat with new readers if it fails.

Consequence: this is a proposed engineering acceptance gate, not evidence of
improvement over a baseline. It has **not run**. E2 reviewers must be different
people. The final gate must cover every function in `gate.accord`; the
`gate_verdict` mock-up below is only one representative function.

### 3. How should stable diagnostic codes work?

Options: prose matching; descriptive string codes; a numeric catalog with named
conditions. Recommend **a small numeric catalog**, `ACC0001` onward, with permanent
meanings and named internal variants. Do not encode severity, source line,
solver, function name, or hash into the code. Never reuse a retired number.

Verified comparisons:

- [rustc JSON](https://doc.rust-lang.org/rustc/json.html) separates code, level,
  spans, child messages, suggestions, and applicability. Some codes are null;
  new fields and enum values may appear. Adopt separation and applicability,
  not a claim that every rustc diagnostic has a numeric code.
- TypeScript [5.9.3's catalog](https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/diagnosticMessages.json)
  stores numeric codes and categories independently of message templates. Its
  [diagnostic types](https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/types.ts)
  carry code, category, location, message chains, and related information. This
  is inspected version behavior, not a verified perpetual stability promise.
- Silk at `f90f584fc3cdd9ed2c5aa6a16a147620e6afe19b` has a
  [central catalog](https://github.com/silk-lang/silk/blob/f90f584fc3cdd9ed2c5aa6a16a147620e6afe19b/compiler/src/diagnostic/Catalog.silk)
  with phase-prefixed codes such as `PAR0001`, a
  [structured renderer](https://github.com/silk-lang/silk/blob/f90f584fc3cdd9ed2c5aa6a16a147620e6afe19b/compiler/src/diagnostic/Report.silk),
  a [catalog validator](https://github.com/silk-lang/silk/blob/f90f584fc3cdd9ed2c5aa6a16a147620e6afe19b/compiler/scripts/DiagnosticCatalog.mjs),
  and [new-user diagnostic tests](https://github.com/silk-lang/silk/blob/f90f584fc3cdd9ed2c5aa6a16a147620e6afe19b/integration/new-user/diagnostics.test.mjs).
  The validator checks unique spellings and declared production coverage gaps.
  A catalog entry is therefore not evidence that every intended producer exists.

Accord has few conditions and agents branch on their meaning, so phase prefixes
buy little today. Descriptive codes are readable but invite accidental renaming.
Numeric identifiers plus documented names keep wording editable and catalog
changes reviewable. Proposed initial conditions, **not allocated codes**:

| Candidate | Condition | Agent action |
| --- | --- | --- |
| ACC0001 | Source rejected | Fix the located syntax or type error, then rerun. |
| ACC0002 | Obligation refuted | Inspect its counterexample or unreachable-row evidence; repair code against approved intent. |
| ACC0003 | Obligation undecided | Branch on typed reason: timeout, solver unknown, disagreement, or execution failure. A retry is not a proof. |
| ACC0004 | Obligation unsupported | Report the unsupported feature; do not silently change the specification. |
| ACC0005 | Example failed | Inspect the expected result or unmet precondition. |
| ACC0006 | Undeclared open case | Present input and both allowed outputs for an intent decision. |
| ACC0007 | Non-equivalent mutant survives | Show the distinguishing input and results; flag contract weakness, without changing standing. |
| ACC0008 | Strength search incomplete | Report undecided and untried counts without treating them as caught. |
| ACC0009 | Solver unavailable | Repair environment; no receipt or proof exists. |
| ACC0010 | Open-case search undecided | Keep standing incomplete and expose the typed reason. |

Consequence: adopt a new receipt schema version and migrate current consumers
together, without a legacy format switch. Keep the current evidence objects as
the source of facts; diagnostics point to those objects rather than duplicating
another evidence ledger. A successful receipt can have no diagnostics and still
must carry its full specification, evidence, intent, and limits.

The result needs a schema version, status, nullable receipt on rejection, and
diagnostics. Each diagnostic needs code, severity, authority, definition identity,
file and source location, evidence reference, typed reason and payload, and a
structured action category. Witness values need explicit types and values,
including enum type and list element type, rather than requiring an agent to
parse Accord expressions. Keep prose as an explanation only. Specify location
units; start with today's 1-based line and nullable columns, not invented spans.

For example, an **illustrative future** open-case record would identify
`ACC0006`, `authority: compiler`, `action: review_specification`, and fields for
typed inputs, two permitted results, and implementation result. This is not
today's JSON. An agent can route it without reading a sentence. Neither this
action nor `ACC0007` authorizes weakening a contract or accepting a freedom.

Keep existing exit meanings: 0 verified, 1 failed, 3 incomplete or unverified,
2 usage error, 4 rejected or unavailable. Once arguments parse, JSON should be
one result document on stdout, including structured rejection; logs go to stderr.
Usage errors keep exit 2 and usage text on stderr. A crash or interruption must
not emit a completed verified result. Unknown schema versions or condition codes
must stop automatic acceptance and retain the raw result for inspection.
Catalog tests must cover unknown-code handling, uniqueness, every producer,
text/JSON parity, exit status, and message edits that preserve machine behavior.

### 4. Which non-Tesota rule?

Options: Cedar's partial decision; Kubernetes' `ResourceNameMatches`; Go's
`BodyAllowedForStatus`. Recommend **Cedar's partial decision abstraction**.

Verified source anchors at the full commit above:

- [Core decision, lines 119–140](https://github.com/cedar-policy/cedar/blob/5af7b8cd78e4e0da3ae23f1715023fff8e62941d/cedar-policy-core/src/authorizer/partial_response.rs#L119).
  Four booleans say whether satisfied forbids, satisfied permits, residual
  permits, and residual forbids are nonempty. Five ordered cases return
  `Some(Deny)`, `Some(Deny)`, `None`, `None`, and `Some(Allow)`.
- [Public wrapper](https://github.com/cedar-policy/cedar/blob/5af7b8cd78e4e0da3ae23f1715023fff8e62941d/cedar-policy/src/api.rs#L1156)
  delegates to the core decision. This is not `concretize`, which treats
  residuals differently.
- [LICENSE](https://github.com/cedar-policy/cedar/blob/5af7b8cd78e4e0da3ae23f1715023fff8e62941d/LICENSE)
  and the source header specify Apache-2.0. A future port must retain applicable
  attribution and notices; this brief does not import Cedar source files.
- [Core `reauthorize` test](https://github.com/cedar-policy/cedar/blob/5af7b8cd78e4e0da3ae23f1715023fff8e62941d/cedar-policy-core/src/authorizer/partial_response.rs#L779)
  checks denial when formerly unknown context makes a forbid apply.
  [FFI tests and helpers](https://github.com/cedar-policy/cedar/blob/5af7b8cd78e4e0da3ae23f1715023fff8e62941d/cedar-policy/src/ffi/is_authorized.rs#L2113)
  check allow, deny, and residual results, including
  `test_authorized_partial_no_resource`,
  `test_authorized_partial_not_authorized_no_resource`, and
  `test_authorized_partial_residual_no_principal_scope`.
  These tests were read, **not executed**. They do not by themselves establish
  exhaustive coverage of the 16 abstract combinations.

Define `f`, `p`, `rp`, `rf` in that order. Map `None` to a domain enum variant
`Unknown`, and `Some` to `Deny` or `Allow`. `Unknown` is a **proved authorization
answer**, not an undecided solver outcome. All 16 combinations are admitted;
collection contents and their consistency are outside the abstraction.

| Ordered source case | Abstract condition after earlier cases fail | Result |
| --- | --- | --- |
| 1 | `f` | Deny |
| 2 | `not p and not rp` | Deny |
| 3 | `not f and rf` | Unknown |
| 4 | `not f and not p and rp and not rf` | Unknown |
| 5 | Remaining inputs: `not f and p and not rf` | Allow |

Consequence: `Bool`, enums, logic, `ensures`, `decide`, and examples suffice.
No records, `Option`, collections API, calls between functions, or pattern
matching are required. The extraction from Cedar's maps to booleans remains
outside the proof and must be stated in the view. Cedar tests precedence and
unknown information in authorization, a useful contrast with verification gates.
The fallback candidates were not independently source-audited because Cedar
passed the fit check. Their names are alternatives, not verified claims about
their implementations or licenses.

### 5. What should stage 7 require of E2?

Options: informal usability feedback; a registered exploratory controlled study;
a powered confirmatory study. Recommend **a registered exploratory controlled
study**, after the readability gate, with an explicit practical pass mark and
uncertainty. The proposal below uses 90 reviewers; feasibility is an owner
decision. If only a convenience sample is available, label the run a pilot and
do not quietly relax the pass mark.

Consequence: stage 7 can deliver a frozen registration without claiming the
review view improves defect detection. Running E2 and accepting its result are
separate owner decisions. No study has been registered or run by this brief.

## Mock-ups grounded in today's checker

These are hand-written layout proposals, not screenshots or output of a new
command. Counts and witnesses come from receipt version 1 on the baseline.
Tables condense paired implications for reading; the final view must retain
every exact clause with its outcome and source location. A table must not be
presented as checked merely because an author paraphrased it.

### `gate_verdict`: one file's proof at settlement

**Intent:** absent from this function. Proposed wording, unchecked and requiring
owner review: “Accept a passed proof; send a new failure back once; report a
repeated failure as no progress; send interrupted or unstarted work to the
operator.” Do not substitute a doc comment for a declared intent silently.

**Inputs:** `outcome: ProofOutcome`, `failure: Text`, `sent: List<Text>`.
`ProofOutcome` is Passed, Failed, Vacuous, TimedOut, Cancelled, or NotStarted.
No `requires`. `failure in sent` uses exact text equality, not similarity of errors.
For a reader, Vacuous means a completed run verified nothing; TimedOut,
Cancelled, and NotStarted describe interruption or absence of a run. Those
input meanings are author-supplied context, not facts this function proves.
SendBack requests another attempt, NoProgress identifies repetition, and
Operator requests human attention. The pure function returns a verdict; it
does not itself perform those actions.

**Specification:** each row below is an “if and only if”, represented by two
proved `ensures` clauses in [gate-rule.accord](../../conformance/programs/tesota/gate-rule.accord).
It describes the promised results, without showing the decision-table body.

| Result | Exactly when | Clauses |
| --- | --- | --- |
| Proved | Outcome is Passed | Lines 29–30 |
| SendBack | Outcome is Failed or Vacuous, and failure is absent from sent | Lines 31–32 |
| NoProgress | Outcome is Failed or Vacuous, and failure is already in sent | Lines 33–34 |
| Operator | Outcome is NotStarted, TimedOut, or Cancelled | Lines 35–36 |

**Evidence:** function standing **verified**. Definition hash
`7f66db67158ec95d9066f577f39b2843464b38620c53040d5926f01ce98fa243`.
Eight postconditions, one table-coverage obligation, and four row-reachability
obligations proved. The first are universal; reachability witnesses are
existential evidence, not proofs that a branch is always selected.

**Executed examples:** Failed, `failure = "exit 1"`, `sent = []` gives SendBack;
the same failure with `sent = ["exit 1"]` gives NoProgress. Both passed.
**Additional row witnesses:** Passed with empty failure and sent reaches Proved;
NotStarted with empty failure and sent reaches Operator. These are the run's
row witnesses, not extra declared examples.

**Counterexamples:** none reported for this function.
**Open cases:** search settled as determined. **Declared freedoms:** none.
**Mutants:** 24 tried, 24 caught, 0 equivalent, 0 survivors, 0 undecided,
0 not tried. This finite mutation sample does not prove contract adequacy.

**Not proved:**

- That these promises match the intended workflow; the function has no intent.
- That a Passed input represents a sound proof, or a failure string identifies
  the same underlying failure across runs.
- That callers populate `sent` correctly, stop or retry correctly, or satisfy
  end-to-end safety or progress properties.
- Absence of all possible implementation mistakes, or correctness of Accord's
  compiler, solvers, and interpreter.

**Program context:** the Tesota folder is **incomplete** because `proof_outcome`
has an undeclared open case and two surviving mutants. Its contract permits
TimedOut as well as Failed on a started unsuccessful run. This does not refute
`gate_verdict`'s own proof, which treats the supplied enum as an input. It limits
what a reviewer can infer about the composition. See the existing
[E1 corpus explanation](e1-open-cases/README.md#material). Do not repair it as
part of stage 7 presentation work.

### `partial_decision`: a partial authorization result

**Intent:** proposed and present in the scratch source, but discarded by today's
parser: “Deny on a satisfied forbid or no possible permit; allow on a satisfied
permit with no possible forbid; otherwise keep the decision unknown.” Unchecked.

**Inputs:** four booleans. `f`: any satisfied forbid; `p`: any satisfied permit;
`rp`: any residual permit; `rf`: any residual forbid. A residual policy still
depends on unknown information. No `requires`. All 16 inputs admitted.

**Specification:** three equivalences, each encoded as two implications.

| Result | Exactly when |
| --- | --- |
| Deny | `f or (not p and not rp)` |
| Allow | `not f and p and not rf` |
| Unknown | `not f and (p or rp) and (rf or not p)` |

**Evidence:** scratch function standing **verified**. Definition hash
`81850447c31394fce25e250b0f7b2a793a88e580985e804f3dd082342e7d5b94`.
Six postconditions, table coverage, and five row-reachability obligations proved.
All 16 declared examples passed. The following selected examples explain the
boundary; the entire truth table is in the reproducible specimen below.

| f | p | rp | rf | Result | Why this case matters |
| --- | --- | --- | --- | --- | --- |
| true | true | true | true | Deny | A satisfied forbid overrides permits and residuals. |
| false | false | false | true | Deny | An unresolved forbid alone cannot supply a permit. |
| false | true | false | true | Unknown | A satisfied permit does not override a possible forbid. |
| false | false | true | false | Unknown | A possible permit is not yet an effective permit. |
| false | true | true | false | Allow | More possible permits do not block a satisfied permit. |

**Counterexamples:** none reported. **Open cases:** determined.
**Declared freedoms:** none. **Mutants:** 26 tried, 24 caught, 2 equivalent,
0 survivors, 0 undecided, 0 not tried. Unknown is an enum value; the function's
answer is determined even when that value is Unknown.

**Not proved:**

- Evaluation, validation, or satisfiability of Cedar policies, or correctness
  of assigning policies to these four collections.
- Equivalence to the upstream executable or wrapper, or behavior of
  `concretize`, reauthorization, errors, and policy diagnostics.
- Whether an application should reject an Unknown decision, request more data,
  or use another policy. The abstraction reports uncertainty; it grants no access.
- Alignment of this contract with the owner's intent, universal contract
  adequacy, or correctness of Accord's trusted components.

### Rules for a faithful stage 7 view

Always show intent as unchecked; show absence explicitly. Keep specification
clauses, example runs, proof outcomes, witnesses, open-case witnesses, and mutant
witnesses distinct. Absence of counterexamples is not a substitute for proved
obligations. `strength: null` means mutation was not applicable, not a perfect
score. `openness: null` means no such search was performed, not determined.
List every declared freedom even if the remaining domain is determined.

Separate **checks that did not settle** from **claims outside the check**.
The former are derived from current outcomes, with reason and location. The
latter include intent alignment, translation boundaries, and trusted components.
Source-specific boundaries need author-supplied text clearly labeled unchecked;
the compiler cannot infer that a Boolean stands for a Cedar collection. Never
claim an assumption was approved or a solver version was receipt-bound when
version 1 carries neither. Record versions alongside this reproduction instead.

Retain intent in the source model and report. Keep proof-cache identity tied to
checked semantics; use a separate review-content identity covering intent,
specification, examples, declared freedoms, and evidence identity. Changing prose
must invalidate a saved review artifact without pretending the proof changed.
No approval store is proposed. No generated prose or model opinion may become
compiler evidence.

## E2 registration — draft, not registered

### Question and hypotheses

Does the review view help people find more true intent defects per minute than
code alone or specification alone, without more false positives or misplaced
confidence? Primary contrasts: review view versus code; review view versus
specification. Analyze code defects and specification defects separately too.

This is a new exploratory study of the bundle. The prior studies above do not
justify assuming a particular effect size. No participant data has been gathered.

### Material and ground truth

Use gate verdict and Cedar partial decision as two domains. Prepare six variants
of each, with anonymous identifiers and neutral formatting. Every arm gets the
same independently written requirement and glossary. The requirement is the
oracle for intent, not the candidate contract, candidate code, or proof result.
The owner and a second domain reader must resolve the answer key before freezing
materials. Do not recruit them as participants.

| Variant per domain | Planted condition | What the oracle must establish |
| --- | --- | --- |
| Clean | Original complete rule and contract | No defect; required negative control. |
| Body error | Flip a result or precedence, retain correct contract | Concrete admitted input violates requirement and proof refutes it. |
| Weak specification | Delete a necessary direction; original body still proves | At least one wrong result is allowed; distinguish a specification defect from an actual body error. |
| Wrong complete specification and body | Gate timeout is treated as success; Cedar a permit overrides an unresolved forbid | Complete but wrong behavior still proves and may have no open case or surviving mutant. |
| Excluded input | A precondition excludes a requirement's important case | Proof passes only on a narrower domain. Provide an excluded-input witness. |
| Misdeclared freedom | An `open when` hides ambiguity in a required case | The body proves, but the declared freedom contradicts the independent requirement. |

Use known semantics to construct these; do not assume a mutation will survive.
Before registration, run every variant and keep actual receipts. Replace a
variant that does not meet its category before freezing it. For example,
deleting one implication may still leave a determined contract because the
remaining clauses imply it. Keep expected outcomes first and planted defects
in both domains. The wrong complete variant tests what evidence cannot detect.
Use the same number and selection policy for examples across variants. For the
wrong complete variant, examples must agree with the wrong candidate contract
so that it still verifies; the independent requirement must remain unchanged.
Disclose these differences and avoid selecting only favorable cases for that arm.

### Reviewers, allocation, and tasks

Propose 90 adult developers, 30 per arm, with basic programming competence;
record experience with contracts, formal verification, authorization, and review.
Stratify randomization by formal-methods experience. Exclude Accord contributors,
material authors, readability-gate readers, and anyone previously shown the
answer key. Recruit and compensate identically across arms.

Use a between-reviewer assignment to prevent transfer between presentations.
Each reviewer gets one variant from each domain, never two versions of the same
rule. Balance the six variants and domain order within each arm using a frozen
random seed and allocation file. This yields five reviews per domain variant per
arm. It is a small exploratory sample; do not treat repeated answers as
independent reviewers or claim adequate power without a precision analysis.

Allow a shared short tutorial and unrelated practice task, then eight minutes
per review. Reviewers identify defects, give an input and expected behavior or
excluded case, identify the affected promise, and record a disposition: accept,
request change, or insufficient information. After each task ask confidence
from 0 to 100 in the disposition. No assistants, execution tools, internet, or
answers from other participants. Record elapsed active task time and whether
they finished; keep confidence collection outside that timer.

### Arms

| Arm | Candidate material, beyond shared requirement and glossary |
| --- | --- |
| Code only | Signature, types, and implementation; no contracts, examples, proof results, or review view. |
| Specification only | Signature, types, `requires`, `ensures`, `open when`, and candidate intent; no body or compiler evidence. |
| Review view | The actual stage 7 per-function report: intent, full specification, examples, current outcomes, counterexamples, openness, freedoms, mutants, and limits; no body. |

All arms may answer “not observable”. For example, specification-only reviewers
cannot see a body-only mistake. Score observability separately and report both
the practical bundle comparison and the subset of intent/specification defects
visible in all arms. Do not interpret this as an isolated causal test of layout:
the review view includes additional compiler evidence. A later ablation study
would be needed to attribute gains to examples or the limits list individually.

### Measures and scoring

- **Primary:** unique true defects found divided by active review minutes,
  aggregated as total true finds over total time per arm. Require a matching
  defect category plus a valid distinguishing or excluded case; deduplicate
  repeated descriptions of one defect. A vague concern earns no true find.
- **False positives:** unsupported defect claims per task, including claims on
  clean controls. Report precision, clean-control rejection rate, and false
  acceptance of defective variants. Correctly declining to judge an unseen body
  is not a false positive.
- **Confidence:** report confidence for correct and incorrect dispositions
  separately; report high-confidence mistakes (at least 80/100). Confidence is
  not an accuracy measure.
- **Secondary:** recall by defect category and domain, time to first valid find,
  completion time, and scope-comprehension answers. Report unsolved tasks and
  requests for information rather than silently dropping them.

Two scorers independently grade anonymized claims against the frozen key, blind
to arm where possible. Record disagreements, adjudication, and additional valid
finds outside the key separately. Use deterministic checks for witness validity
where Accord can express the claim. A verifier cannot adjudicate whether the
independent requirement is intended.

### Analysis, stopping, and proposed pass mark

Freeze allocation, scoring script, analysis, and thresholds before recruitment.
Run a disjoint logistics pilot and a simulation of interval precision first;
publish those choices before registration, without using E2 outcomes to choose
sample size. Fix the final sample size then. With the proposed N=90, a wide
interval is an inconclusive result, not a reason to collect until significance.

Analyze task-level observations with reviewer clustering and domain/variant
strata. Predeclare a stratified reviewer bootstrap with 10,000 resamples and
Bonferroni-adjusted two-sided 97.5% intervals for each of the two primary rate
ratios, targeting nominal 95% simultaneous coverage. Apply the same adjustment
to the pair of false-positive differences. Define zero-baseline
handling in the frozen script: report the absolute rate difference and mark
the ratio unavailable, rather than manufacture an infinite improvement.

Proposed practical pass mark, chosen for product value rather than borrowed
from a paper:

1. Review-view rate is at least 25% higher than **each** baseline, and each
   simultaneous interval's lower bound is above 1.
2. The upper simultaneous confidence bound for excess false positives is at
   most 0.10 per task against each baseline.
3. Both domains show a positive observed rate difference. Specification-defect
   recall does not fall below either baseline on the common-observability subset.
4. High-confidence wrong acceptance is no higher in the review arm than either
   baseline. Report uncertainty; this is a guardrail, not a proven calibration gain.

Failure of any criterion means the proposed pass mark was not met. An interval
that cannot resolve the contrast is inconclusive, not proof of harm or benefit.
Report absolute rates as well as ratios. Present all exclusions and all arms.

Predeclare exclusions: lack of consent or eligibility, duplicate participation,
and unusable timing due to a recorded technical interruption. Do not exclude
slow, wrong, low-confidence, or unfinished reviews. Do not use an arbitrary
fast-response cutoff after inspecting outcomes. For interrupted tasks retain
available accuracy data and report missing timing. Stop at the frozen sample
or a named recruitment deadline, then report any shortfall.

### Registration artifact and permissions

Before running E2, publish an immutable, timestamped registration containing
the materials' hashes, Accord revision, solver versions, randomization,
answer-key commitment, scoring rules, analysis, pass mark, recruitment limit,
exclusions, and consent/data-retention policy. Keep the answer key hidden from
participants until collection closes; commit its hash before the run and disclose
it afterward. Select the registration venue with the owner. No publication,
recruitment, collection, or human approval has happened in this task.

## Proposed implementation slices after owner decisions

These are future work, not changes in this branch. Each slice writes expected
conformance outcomes before compiler changes. Preserve fail-closed standing.
Use existing report, receipt, identity, and diagnostic paths rather than a
parallel checking subsystem. Proposed test names below must be added by their
slice; they do not exist today.

| Slice | Deliverable and boundary | Check ending the slice |
| --- | --- | --- |
| 1. Cedar target | Add the attributed Boolean port, 16 expected examples, and planted precedence/result faults. Keep collection extraction out of scope. | `accord check conformance/programs/cedar --json` verifies the reference; `cargo test --workspace --locked` confirms the faults have their prewritten expected refutations and witnesses. |
| 2. Retained intent and review identity | Retain optional intent and exact specification clauses. Separate proof identity from review-content identity. Absence is explicit. | `cargo test --test review_identity --locked`: changing intent changes review identity, preserves reusable proof keys, and cannot create approval; specification and example edits change the appropriate identities. |
| 3. Structured result and codes | One catalog and new receipt schema; migrate consumers, including rejection and solver setup errors. No legacy switch. | `cargo test --test diagnostics --locked`: every catalog condition has a producer case, duplicate and unknown codes are handled, payloads round-trip, prose edits do not affect routing, and exit codes agree with status. |
| 4. Human projection and limits | Render the whole gate rule and Cedar, including every clause, example, gap, declared freedom, and survivor. No generated opinion as evidence. | `cargo test --test review --locked`: terminal/JSON parity on verified, refuted, timeout, unsupported, disagreement, open, freedom, survivor, empty and rejected inputs. Actual outputs must match the same evidence objects. Then run the three-reader gate with frozen answers for both targets. |
| 5. E2 registration | Freeze corpus, ground truth, arms, pilot findings, allocation and scoring. Resolve the five owner decisions and registration venue. | Add and run `python docs/design/e2-human-review/validate.py` and `python docs/design/e2-human-review/score.py --self-test`: reproduce corpus receipts, artifact hashes, allocation balance, and synthetic scoring expectations. A timestamped registration predates the first E2 participant. These proposed scripts do not exist yet. |

Before any implementation push, run the repository gates:
`cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and
`cargo test --workspace --locked` with the pinned solvers. The human readability
gate and E2 outcomes remain separate from those automated checks.

Unresolved owner questions: terminal first or page too; the readability gate and
its readers; numeric codes and schema change; Cedar's abstraction boundary;
E2's feasible sample, pass mark, and registration venue.

## Reproduction and research limits

Commands actually run for the mock-ups, with the built binary also callable as
`accord` on PATH:

```sh
bash scripts/install-solvers.sh <temporary-solver-folder>
# Export ACCORD_Z3 and ACCORD_CVC5 as printed by that script.
cargo run --locked -- check conformance/programs/tesota --json
# exit 3: folder incomplete; gate_verdict verified, 13 obligations, 2 examples.
# proof_outcome: an open case, 2 surviving mutants; no repair made.
accord check <temporary-cedar-specimen.accord> --json
# exit 0: verified; 12 obligations, 16 examples; 26 mutants tried,
# 24 caught, 2 equivalent, no survivors or undecided checks.
```

The install script verified its pinned archive checksums. Versions printed:
Z3 5.3.3, build `430d19e697e2ec9bad79ff210d41b4a7a5fcfa3e`;
cvc5 1.4.2, commit `282d5e0`. The first Tesota run used a fresh worktree ledger.
Solver witness choices and reuse flags can differ on reruns; compare meanings
and counts, not witness spelling. Raw output was kept in temporary files during
research. Version 1 does not embed these solver versions in its receipt.

To repeat the Cedar check, save this entire block to a temporary `.accord` file
and check it with the same binary. Expected results were fixed before the run.
The table abstraction follows the inspected source, and all 16 examples run
through Accord's interpreter. This is not an upstream executable differential
test. The specimen is research material, not a committed conformance port.

```accord
enum PartialDecision { Deny, Allow, Unknown }
fn partial_decision(f: Bool, p: Bool, rp: Bool, rf: Bool) -> PartialDecision
    intent "Deny on a satisfied forbid or no possible permit; allow on a satisfied permit with no possible forbid; otherwise keep the decision unknown."
    ensures result == Deny implies f or (not p and not rp)
    ensures f or (not p and not rp) implies result == Deny
    ensures result == Allow implies not f and p and not rf
    ensures not f and p and not rf implies result == Allow
    ensures result == Unknown implies not f and (p or rp) and (rf or not p)
    ensures not f and (p or rp) and (rf or not p) implies result == Unknown
decide {
    | f => Deny
    | not p and not rp => Deny
    | not f and rf => Unknown
    | not f and not p and rp and not rf => Unknown
    | otherwise => Allow
}
example partial_decision(f: false, p: false, rp: false, rf: false) == Deny
example partial_decision(f: false, p: false, rp: false, rf: true) == Deny
example partial_decision(f: false, p: false, rp: true, rf: false) == Unknown
example partial_decision(f: false, p: false, rp: true, rf: true) == Unknown
example partial_decision(f: false, p: true, rp: false, rf: false) == Allow
example partial_decision(f: false, p: true, rp: false, rf: true) == Unknown
example partial_decision(f: false, p: true, rp: true, rf: false) == Allow
example partial_decision(f: false, p: true, rp: true, rf: true) == Unknown
example partial_decision(f: true, p: false, rp: false, rf: false) == Deny
example partial_decision(f: true, p: false, rp: false, rf: true) == Deny
example partial_decision(f: true, p: false, rp: true, rf: false) == Deny
example partial_decision(f: true, p: false, rp: true, rf: true) == Deny
example partial_decision(f: true, p: true, rp: false, rf: false) == Deny
example partial_decision(f: true, p: true, rp: false, rf: true) == Deny
example partial_decision(f: true, p: true, rp: true, rf: false) == Deny
example partial_decision(f: true, p: true, rp: true, rf: true) == Deny
```

Method: a bounded primary-source search starting with the supplied paper and
issue, then studies of specification comprehension, examples, counterexample
explanation, and review displays; official documentation for each named tool;
and targeted issue/discussion searches for user-reported difficulties. Opened
original studies rather than adopting summaries. Cedar was checked out at the
requested commit; Silk was inspected at the hash cited above. TypeScript's
unversioned source URLs returned 404, so the comparison uses 5.9.3 explicitly.
Some F* tutorial URLs were inaccessible; the working official chapter is cited.

This is not a systematic literature review. The search does not establish that
these are the only relevant studies, that all tool users face these problems,
or that a reported historical defect persists. No direct comparative study of
all eight tools was verified. No isolated causal evidence for the “not proved”
list was found in the searched sources. Cedar's Rust tests, an upstream
differential execution, human readability, and E2 were not run. The scratch
port establishes language fit only. No compiler implementation, release, PR,
or merge is part of this brief.
