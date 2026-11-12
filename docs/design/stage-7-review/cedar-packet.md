# Cedar readability packet

Read only this packet and the glossary below; do not open source or the answer key.
This is a readability check, not E2. No human result is recorded yet. The text
below is generated from real checker output; only cache-use markers are removed.
Verified means checked against the written specification, not approved intent.
An example is one executed call; a row witness establishes existence, not a
universal guarantee. The functions return values and have no external effects.
Text membership is exact equality. Proved/SendBack/NoProgress/Operator are gate
verdicts; Passed/Failed/Vacuous/TimedOut/Cancelled/NotStarted describe proof-run
outcomes. Vacuous means nothing was verified. Cedar residuals depend on unknown
information; Unknown is an authorization answer, not solver uncertainty.

Recorded command: `accord check conformance/programs/cedar`. Display: unwrapped plain terminal
text; no graphical terminal viewport was captured. Data: the named conformance
program. Interaction: run the check and read its output.

Copyright Cedar Contributors. Modified Boolean port of Cedar's partial
authorization decision, licensed under [Apache-2.0](../../../LICENSE-APACHE);
see the retained [notice](../../../conformance/programs/cedar/NOTICE).

```text
function     fn partial_decision(satisfied_forbid: Bool, satisfied_permit: Bool, residual_permit: Bool, residual_forbid: Bool) -> PartialDecision (partial-decision.accord)
review_hash  f92c4f8f341c7109abc0d2edcdd42e85612146acefdf4304009015eaefc21524
type         enum PartialDecision { Deny, Allow, Unknown }
intent       partial_decision (partial-decision.accord) (unchecked): Deny on a satisfied forbid or no possible permit; allow on a satisfied permit with no possible forbid; otherwise keep the decision unknown.
specification line 21: ensures result == Deny implies satisfied_forbid or (not satisfied_permit and not residual_permit)
specification line 22: ensures satisfied_forbid or (not satisfied_permit and not residual_permit) implies result == Deny
specification line 23: ensures result == Allow implies not satisfied_forbid and satisfied_permit and not residual_forbid
specification line 24: ensures not satisfied_forbid and satisfied_permit and not residual_forbid implies result == Allow
specification line 25: ensures result == Unknown implies not satisfied_forbid and (satisfied_permit or residual_permit) and (residual_forbid or not satisfied_permit)
specification line 26: ensures not satisfied_forbid and (satisfied_permit or residual_permit) and (residual_forbid or not satisfied_permit) implies result == Unknown
proved       partial_decision (line 21): ensures result == Deny implies satisfied_forbid or (not satisfied_permit and not residual_permit)
proved       partial_decision (line 22): ensures satisfied_forbid or (not satisfied_permit and not residual_permit) implies result == Deny
proved       partial_decision (line 23): ensures result == Allow implies not satisfied_forbid and satisfied_permit and not residual_forbid
proved       partial_decision (line 24): ensures not satisfied_forbid and satisfied_permit and not residual_forbid implies result == Allow
proved       partial_decision (line 25): ensures result == Unknown implies not satisfied_forbid and (satisfied_permit or residual_permit) and (residual_forbid or not satisfied_permit)
proved       partial_decision (line 26): ensures not satisfied_forbid and (satisfied_permit or residual_permit) and (residual_forbid or not satisfied_permit) implies result == Unknown
proved       partial_decision (line 27): every input reaches a row
proved       partial_decision (line 28): some admitted input reaches this row first (existential)
             reached with satisfied_forbid = true, satisfied_permit = false, residual_permit = false, residual_forbid = false
proved       partial_decision (line 29): some admitted input reaches this row first (existential)
             reached with satisfied_forbid = false, satisfied_permit = false, residual_permit = false, residual_forbid = false
proved       partial_decision (line 30): some admitted input reaches this row first (existential)
             reached with satisfied_forbid = false, satisfied_permit = true, residual_permit = false, residual_forbid = true
proved       partial_decision (line 31): some admitted input reaches this row first (existential)
             reached with satisfied_forbid = false, satisfied_permit = false, residual_permit = true, residual_forbid = false
proved       partial_decision (line 32): some admitted input reaches this row first (existential)
             reached with satisfied_forbid = false, satisfied_permit = true, residual_permit = false, residual_forbid = false
counterexamples none reported (this alone is not a proof)
passed       partial_decision (line 35): example partial_decision(satisfied_forbid: false, satisfied_permit: false, residual_permit: false, residual_forbid: false) == Deny
passed       partial_decision (line 36): example partial_decision(satisfied_forbid: false, satisfied_permit: false, residual_permit: false, residual_forbid: true) == Deny
passed       partial_decision (line 37): example partial_decision(satisfied_forbid: false, satisfied_permit: false, residual_permit: true, residual_forbid: false) == Unknown
passed       partial_decision (line 38): example partial_decision(satisfied_forbid: false, satisfied_permit: false, residual_permit: true, residual_forbid: true) == Unknown
passed       partial_decision (line 39): example partial_decision(satisfied_forbid: false, satisfied_permit: true, residual_permit: false, residual_forbid: false) == Allow
passed       partial_decision (line 40): example partial_decision(satisfied_forbid: false, satisfied_permit: true, residual_permit: false, residual_forbid: true) == Unknown
passed       partial_decision (line 41): example partial_decision(satisfied_forbid: false, satisfied_permit: true, residual_permit: true, residual_forbid: false) == Allow
passed       partial_decision (line 42): example partial_decision(satisfied_forbid: false, satisfied_permit: true, residual_permit: true, residual_forbid: true) == Unknown
passed       partial_decision (line 43): example partial_decision(satisfied_forbid: true, satisfied_permit: false, residual_permit: false, residual_forbid: false) == Deny
passed       partial_decision (line 44): example partial_decision(satisfied_forbid: true, satisfied_permit: false, residual_permit: false, residual_forbid: true) == Deny
passed       partial_decision (line 45): example partial_decision(satisfied_forbid: true, satisfied_permit: false, residual_permit: true, residual_forbid: false) == Deny
passed       partial_decision (line 46): example partial_decision(satisfied_forbid: true, satisfied_permit: false, residual_permit: true, residual_forbid: true) == Deny
passed       partial_decision (line 47): example partial_decision(satisfied_forbid: true, satisfied_permit: true, residual_permit: false, residual_forbid: false) == Deny
passed       partial_decision (line 48): example partial_decision(satisfied_forbid: true, satisfied_permit: true, residual_permit: false, residual_forbid: true) == Deny
passed       partial_decision (line 49): example partial_decision(satisfied_forbid: true, satisfied_permit: true, residual_permit: true, residual_forbid: false) == Deny
passed       partial_decision (line 50): example partial_decision(satisfied_forbid: true, satisfied_permit: true, residual_permit: true, residual_forbid: true) == Deny
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     partial_decision: 23 mutants: 21 caught, 2 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     partial_decision (partial-decision.accord): verified
function     fn some_policy(effects: List<Effect>, evaluations: List<Evaluation>, effect: Effect, evaluation: Evaluation) -> Bool (policy-set.accord)
review_hash  78c42516fa1c4f4227b6bf67acaf7984771baa4c2bd5b82f0f02d788581aef48
type         enum Effect { Permit, Forbid }
type         enum Evaluation { Satisfied, NotSatisfied, Residual, Error }
intent       some_policy (policy-set.accord) (unchecked): Some policy with this effect evaluated to this result.
specification line 30: requires length(evaluations) == length(effects)
specification line 31: ensures result == (any k in indices(effects): effects[k] == effect and evaluations[k] == evaluation)
proved       some_policy (line 31): ensures result == (any k in indices(effects): effects[k] == effect and evaluations[k] == evaluation)
proved       some_policy (line 31): `effects[k]` is within its list
proved       some_policy (line 31): `evaluations[k]` is within its list
proved       some_policy (line 33): `effects[k]` is within its list
proved       some_policy (line 33): `evaluations[k]` is within its list
counterexamples none reported (this alone is not a proof)
examples     none declared
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     some_policy: 3 mutants: 3 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   list_assumptions: Every list is assumed to hold at most 4,294,967,295 elements, as a TypeScript array does, and `count` is known to the solvers only through trusted axioms: its bounds, when it is zero and when it is the length.
not_proved   no_examples: No declared example was executed for this function.
not_proved   admitted_domain: Guarantees apply only to inputs meeting requires; no behavior is promised for excluded inputs.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     some_policy (policy-set.accord): verified
function     fn partial_authorization(effects: List<Effect>, evaluations: List<Evaluation>) -> PartialDecision (policy-set.accord)
review_hash  8b4555fe76c23ef271ef357e8b7c55873f8eaba5bf7cbe3cb760bbf17095483d
type         enum Effect { Permit, Forbid }
type         enum Evaluation { Satisfied, NotSatisfied, Residual, Error }
type         enum PartialDecision { Deny, Allow, Unknown }
intent       partial_authorization (policy-set.accord) (unchecked): Deny on a satisfied forbid or with no permit that can hold; allow on a satisfied permit with no forbid that can hold; otherwise unknown.
specification line 41: requires length(evaluations) == length(effects)
specification line 42: ensures (result == Deny) == ((any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied) or not ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) or (any k in indices(effects): effects[k] == Permit and evaluations[k] == Residual)))
specification line 43: ensures (result == Allow) == (not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied) and (any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Residual))
specification line 44: ensures (all k in indices(effects): evaluations[k] != Residual) implies (result == Allow) == ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied))
proved       partial_authorization (line 42): ensures (result == Deny) == ((any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied) or not ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) or (any k in indices(effects): effects[k] == Permit and evaluations[k] == Residual)))
proved       partial_authorization (line 42): `effects[k]` is within its list
proved       partial_authorization (line 42): `evaluations[k]` is within its list
proved       partial_authorization (line 42): `effects[k]` is within its list
proved       partial_authorization (line 42): `evaluations[k]` is within its list
proved       partial_authorization (line 42): `effects[k]` is within its list
proved       partial_authorization (line 42): `evaluations[k]` is within its list
proved       partial_authorization (line 43): ensures (result == Allow) == (not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied) and (any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Residual))
proved       partial_authorization (line 43): `effects[k]` is within its list
proved       partial_authorization (line 43): `evaluations[k]` is within its list
proved       partial_authorization (line 43): `effects[k]` is within its list
proved       partial_authorization (line 43): `evaluations[k]` is within its list
proved       partial_authorization (line 43): `effects[k]` is within its list
proved       partial_authorization (line 43): `evaluations[k]` is within its list
proved       partial_authorization (line 44): ensures (all k in indices(effects): evaluations[k] != Residual) implies (result == Allow) == ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied))
proved       partial_authorization (line 44): `evaluations[k]` is within its list
proved       partial_authorization (line 44): `effects[k]` is within its list
proved       partial_authorization (line 44): `evaluations[k]` is within its list
proved       partial_authorization (line 44): `effects[k]` is within its list
proved       partial_authorization (line 44): `evaluations[k]` is within its list
proved       partial_authorization (line 47): `some_policy(effects: effects, evaluations: evaluations, effect: Forbid, evaluation: Satisfied)` meets `some_policy`'s requires
proved       partial_authorization (line 48): `some_policy(effects: effects, evaluations: evaluations, effect: Permit, evaluation: Satisfied)` meets `some_policy`'s requires
proved       partial_authorization (line 49): `some_policy(effects: effects, evaluations: evaluations, effect: Permit, evaluation: Residual)` meets `some_policy`'s requires
proved       partial_authorization (line 50): `some_policy(effects: effects, evaluations: evaluations, effect: Forbid, evaluation: Residual)` meets `some_policy`'s requires
counterexamples none reported (this alone is not a proof)
passed       partial_authorization (line 75): example partial_authorization(effects: [Permit, Forbid], evaluations: [Satisfied, Residual]) == Unknown
passed       partial_authorization (line 76): example partial_authorization(effects: [Permit, Forbid], evaluations: [Satisfied, NotSatisfied]) == Allow
passed       partial_authorization (line 77): example partial_authorization(effects: [Permit], evaluations: [Residual]) == Unknown
passed       partial_authorization (line 78): example partial_authorization(effects: [Forbid], evaluations: [Residual]) == Deny
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     partial_authorization: 0 mutants: 0 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   list_assumptions: Every list is assumed to hold at most 4,294,967,295 elements, as a TypeScript array does, and `count` is known to the solvers only through trusted axioms: its bounds, when it is zero and when it is the length.
not_proved   admitted_domain: Guarantees apply only to inputs meeting requires; no behavior is promised for excluded inputs.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     partial_authorization (policy-set.accord): verified
function     fn is_authorized(effects: List<Effect>, evaluations: List<Evaluation>) -> Decision (policy-set.accord)
review_hash  ae601e0acac29091dfce90213c371028cff96a666872cb9db8a67607c12c0b83
type         enum Effect { Permit, Forbid }
type         enum Evaluation { Satisfied, NotSatisfied, Residual, Error }
type         enum Decision { Allow, Deny }
intent       is_authorized (policy-set.accord) (unchecked): Allow only when some permit is satisfied and no forbid is; deny by default, and skip a policy whose evaluation failed.
specification line 59: requires length(evaluations) == length(effects)
specification line 60: ensures (result == Allow) == ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied))
specification line 61: ensures (all k in indices(effects): effects[k] == Permit) implies (result == Allow) == (any k in indices(effects): evaluations[k] == Satisfied)
proved       is_authorized (line 60): ensures (result == Allow) == ((any k in indices(effects): effects[k] == Permit and evaluations[k] == Satisfied) and not (any k in indices(effects): effects[k] == Forbid and evaluations[k] == Satisfied))
proved       is_authorized (line 60): `effects[k]` is within its list
proved       is_authorized (line 60): `evaluations[k]` is within its list
proved       is_authorized (line 60): `effects[k]` is within its list
proved       is_authorized (line 60): `evaluations[k]` is within its list
proved       is_authorized (line 61): ensures (all k in indices(effects): effects[k] == Permit) implies (result == Allow) == (any k in indices(effects): evaluations[k] == Satisfied)
proved       is_authorized (line 61): `effects[k]` is within its list
proved       is_authorized (line 61): `evaluations[k]` is within its list
proved       is_authorized (line 63): `some_policy(effects: effects, evaluations: evaluations, effect: Permit, evaluation: Satisfied)` meets `some_policy`'s requires
proved       is_authorized (line 64): `some_policy(effects: effects, evaluations: evaluations, effect: Forbid, evaluation: Satisfied)` meets `some_policy`'s requires
counterexamples none reported (this alone is not a proof)
passed       is_authorized (line 71): example is_authorized(effects: [], evaluations: []) == Deny
passed       is_authorized (line 72): example is_authorized(effects: [Permit, Forbid], evaluations: [Satisfied, Error]) == Allow
passed       is_authorized (line 73): example is_authorized(effects: [Permit, Forbid], evaluations: [Satisfied, Satisfied]) == Deny
passed       is_authorized (line 74): example is_authorized(effects: [Permit, Permit], evaluations: [Error, Residual]) == Deny
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     is_authorized: 2 mutants: 2 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   list_assumptions: Every list is assumed to hold at most 4,294,967,295 elements, as a TypeScript array does, and `count` is known to the solvers only through trusted axioms: its bounds, when it is zero and when it is the length.
not_proved   admitted_domain: Guarantees apply only to inputs meeting requires; no behavior is promised for excluded inputs.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     is_authorized (policy-set.accord): verified
standing     the folder: verified (51 of 51 obligations proved, 51 reused from the ledger; 24 of 24 examples passed)
```

## Questions

1. What does each of the four inputs mean, and which combinations are admitted?
2. With every flag true, what is the result?
3. With no satisfied or residual permit but a residual forbid, what is the result?
4. With a satisfied permit and residual forbid but no satisfied forbid, what is the result?
5. With only a residual permit, what is the result?
6. With a satisfied permit, a residual permit, and no forbids, what is the result?
7. Does Unknown mean the solvers did not settle? Does it make this function's contract open?
8. Does this prove that policies were evaluated into the right collections, or that Unknown should grant access? Explain.

Return answers numbered 1–8 and any unclear wording.
