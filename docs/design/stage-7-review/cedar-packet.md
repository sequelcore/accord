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
standing     the folder: verified (12 of 12 obligations proved, 12 reused from the ledger; 16 of 16 examples passed)
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
