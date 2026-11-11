# Gate readability packet

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

Recorded command: `accord check conformance/programs/tesota`. Display: unwrapped plain terminal
text; no graphical terminal viewport was captured. Data: the named conformance
program. Interaction: run the check and read its output.

```text
function     fn verification_required(changed: Bool, selected: Bool) -> Bool (gate.accord)
review_hash  5d57a74ff6e0100d82f65865e28025e3e821dfe26089666f4dff70603e44885b
intent       verification_required (gate.accord) (unchecked): Require verification after changed files or a selected design check.
specification line 15: ensures result implies changed or selected
specification line 16: ensures changed or selected implies result
proved       verification_required (line 15): ensures result implies changed or selected
proved       verification_required (line 16): ensures changed or selected implies result
counterexamples none reported (this alone is not a proof)
passed       verification_required (line 81): example verification_required(changed: false, selected: true) == true
passed       verification_required (line 82): example verification_required(changed: false, selected: false) == false
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     verification_required: 1 mutant: 1 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     verification_required (gate.accord): verified
function     fn tool_may_change_files(tool_name: Text, native_question: Bool, read_only: Bool) -> Bool (gate.accord)
review_hash  a9d338cf7716803c967548115538deee198c9f37319c66b2fb31347633edcaa8
intent       tool_may_change_files (gate.accord) (unchecked): A started tool may change files unless it asks a native question, runs read-only, or is a trusted reader.
specification line 24: ensures result implies not native_question and not read_only and not tool_name in ["read", "grep", "find", "ls", "update_tasks"]
specification line 25: ensures not native_question and not read_only and not tool_name in ["read", "grep", "find", "ls", "update_tasks"] implies result
proved       tool_may_change_files (line 24): ensures result implies not native_question and not read_only and not tool_name in ["read", "grep", "find", "ls", "update_tasks"]
proved       tool_may_change_files (line 25): ensures not native_question and not read_only and not tool_name in ["read", "grep", "find", "ls", "update_tasks"] implies result
counterexamples none reported (this alone is not a proof)
passed       tool_may_change_files (line 79): example tool_may_change_files(tool_name: "ls", native_question: false, read_only: false) == false
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     tool_may_change_files: 3 mutants: 3 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     tool_may_change_files (gate.accord): verified
function     fn gate_verdict(outcome: ProofOutcome, failure: Text, sent: List<Text>) -> GateVerdict (gate.accord)
review_hash  3e0b15bfab162e49fa20472073d4848778792e300d5809e797888cc0b5e2a6d3
type         enum ProofOutcome { Passed, Failed, Vacuous, TimedOut, Cancelled, NotStarted }
type         enum GateVerdict { Proved, SendBack, NoProgress, Operator }
intent       gate_verdict (gate.accord) (unchecked): Accept a passed proof; send a failed or vacuous proof back once per failure; identify repetition as no progress; ask the operator about interrupted or unstarted work.
specification line 32: ensures result == Proved implies outcome == Passed
specification line 33: ensures outcome == Passed implies result == Proved
specification line 34: ensures result == SendBack implies (outcome == Failed or outcome == Vacuous) and not failure in sent
specification line 35: ensures (outcome == Failed or outcome == Vacuous) and not failure in sent implies result == SendBack
specification line 36: ensures result == NoProgress implies (outcome == Failed or outcome == Vacuous) and failure in sent
specification line 37: ensures (outcome == Failed or outcome == Vacuous) and failure in sent implies result == NoProgress
specification line 38: ensures result == Operator implies outcome == NotStarted or outcome == TimedOut or outcome == Cancelled
specification line 39: ensures outcome == NotStarted or outcome == TimedOut or outcome == Cancelled implies result == Operator
proved       gate_verdict (line 32): ensures result == Proved implies outcome == Passed
proved       gate_verdict (line 33): ensures outcome == Passed implies result == Proved
proved       gate_verdict (line 34): ensures result == SendBack implies (outcome == Failed or outcome == Vacuous) and not failure in sent
proved       gate_verdict (line 35): ensures (outcome == Failed or outcome == Vacuous) and not failure in sent implies result == SendBack
proved       gate_verdict (line 36): ensures result == NoProgress implies (outcome == Failed or outcome == Vacuous) and failure in sent
proved       gate_verdict (line 37): ensures (outcome == Failed or outcome == Vacuous) and failure in sent implies result == NoProgress
proved       gate_verdict (line 38): ensures result == Operator implies outcome == NotStarted or outcome == TimedOut or outcome == Cancelled
proved       gate_verdict (line 39): ensures outcome == NotStarted or outcome == TimedOut or outcome == Cancelled implies result == Operator
proved       gate_verdict (line 40): every input reaches a row
proved       gate_verdict (line 41): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Passed, failure = "", sent = []
proved       gate_verdict (line 42): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.NotStarted, failure = "", sent = []
proved       gate_verdict (line 43): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Failed, failure = "!0!", sent = ["!0!"]
proved       gate_verdict (line 44): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Failed, failure = "!0!", sent = []
counterexamples none reported (this alone is not a proof)
passed       gate_verdict (line 75): example gate_verdict(outcome: Failed, failure: "exit 1", sent: []) == SendBack
passed       gate_verdict (line 76): example gate_verdict(outcome: Failed, failure: "exit 1", sent: ["exit 1"]) == NoProgress
passed       gate_verdict (line 83): example gate_verdict(outcome: Passed, failure: "", sent: []) == Proved
passed       gate_verdict (line 84): example gate_verdict(outcome: TimedOut, failure: "", sent: []) == Operator
passed       gate_verdict (line 85): example gate_verdict(outcome: Vacuous, failure: "no checks", sent: []) == SendBack
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     gate_verdict: 25 mutants: 25 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     gate_verdict (gate.accord): verified
function     fn command_verdict(outcome: ProofOutcome, failure: Text, sent: List<Text>, preexisting: Bool) -> GateVerdict (gate.accord)
review_hash  7ec81e54e3dacb9cb311d708ff09824dcdec11cc15b2bf61ba2e6b119d8f6c53
type         enum ProofOutcome { Passed, Failed, Vacuous, TimedOut, Cancelled, NotStarted }
type         enum GateVerdict { Proved, SendBack, NoProgress, Operator }
intent       command_verdict (gate.accord) (unchecked): Treat an already present command failure as the operator's; otherwise accept passed proofs, send failures back once, and identify interrupted work or repetition.
specification line 50: ensures result == Proved implies outcome == Passed
specification line 51: ensures outcome == Passed implies result == Proved
specification line 52: ensures result == SendBack implies not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and not failure in sent
specification line 53: ensures not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and not failure in sent implies result == SendBack
specification line 54: ensures result == NoProgress implies not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and failure in sent
specification line 55: ensures not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and failure in sent implies result == NoProgress
specification line 56: ensures result == Operator implies (preexisting and outcome == Failed) or outcome == NotStarted or outcome == TimedOut or outcome == Cancelled
specification line 57: ensures (preexisting and outcome == Failed) or outcome == NotStarted or outcome == TimedOut or outcome == Cancelled implies result == Operator
proved       command_verdict (line 50): ensures result == Proved implies outcome == Passed
proved       command_verdict (line 51): ensures outcome == Passed implies result == Proved
proved       command_verdict (line 52): ensures result == SendBack implies not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and not failure in sent
proved       command_verdict (line 53): ensures not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and not failure in sent implies result == SendBack
proved       command_verdict (line 54): ensures result == NoProgress implies not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and failure in sent
proved       command_verdict (line 55): ensures not (preexisting and outcome == Failed) and (outcome == Failed or outcome == Vacuous) and failure in sent implies result == NoProgress
proved       command_verdict (line 56): ensures result == Operator implies (preexisting and outcome == Failed) or outcome == NotStarted or outcome == TimedOut or outcome == Cancelled
proved       command_verdict (line 57): ensures (preexisting and outcome == Failed) or outcome == NotStarted or outcome == TimedOut or outcome == Cancelled implies result == Operator
proved       command_verdict (line 58): every input reaches a row
proved       command_verdict (line 59): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Failed, failure = "", sent = [], preexisting = true
proved       command_verdict (line 60): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Passed, failure = "", sent = [], preexisting = false
proved       command_verdict (line 61): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.NotStarted, failure = "", sent = [], preexisting = false
proved       command_verdict (line 62): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Failed, failure = "!0!", sent = ["!0!"], preexisting = false
proved       command_verdict (line 63): some admitted input reaches this row first (existential)
             reached with outcome = ProofOutcome.Failed, failure = "!0!", sent = [], preexisting = false
counterexamples none reported (this alone is not a proof)
passed       command_verdict (line 77): example command_verdict(outcome: Failed, failure: "exit 1", sent: [], preexisting: true) == Operator
passed       command_verdict (line 86): example command_verdict(outcome: Vacuous, failure: "no checks", sent: [], preexisting: true) == SendBack
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     command_verdict: 33 mutants: 32 caught, 1 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     command_verdict (gate.accord): verified
function     fn keeps_working(verdicts: List<GateVerdict>) -> Bool (gate.accord)
review_hash  219fdf0a9849b39b543e8d73b61ce36d1ded8a38cc4df4902a6bdffb203687d5
type         enum GateVerdict { Proved, SendBack, NoProgress, Operator }
intent       keeps_working (gate.accord) (unchecked): Continue while at least one verdict requests another attempt.
specification line 69: ensures result implies SendBack in verdicts
specification line 70: ensures SendBack in verdicts implies result
proved       keeps_working (line 69): ensures result implies SendBack in verdicts
proved       keeps_working (line 70): ensures SendBack in verdicts implies result
counterexamples none reported (this alone is not a proof)
passed       keeps_working (line 78): example keeps_working(verdicts: [Proved, SendBack]) == true
passed       keeps_working (line 87): example keeps_working(verdicts: [Proved, Operator]) == false
open_cases   none outside declared freedoms (search settled)
freedoms     none declared
strength     keeps_working: 1 mutant: 1 caught, 0 equivalent, 0 surviving, 0 undecided
not_proved   intent_alignment: The compiler does not prove that the specification captures the intended requirement.
not_proved   integration: Callers, external interpretation of inputs, and surrounding system behavior were not checked.
not_proved   trusted_components: The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.
not_proved   mutation_sample: Only the reported finite sample of mutants was checked; contract adequacy is not proved.
standing     keeps_working (gate.accord): verified

Program context: the checked folder is incomplete.
Related function proof_outcome: incomplete; open_case, surviving_mutants.
```

## Questions

1. With changed=false and selected=true, is verification required? What about both false?
2. Can ls change files under this rule? Can an unlisted tool when neither native_question nor read_only holds?
3. What do Failed with a new failure and Failed with a repeated failure return?
4. What do Passed, TimedOut, and NotStarted return from gate_verdict?
5. What do preexisting Failed and preexisting Vacuous with a new failure return from command_verdict?
6. Does keeps_working continue for [Proved, Operator]? For [Proved, SendBack]?
7. Does this prove that a Passed input is a sound proof or that the whole checked program is verified? Explain.
8. Does determined plus verified establish that the specification captures the intended workflow? Explain.

Return answers numbered 1–8 and any unclear wording.
