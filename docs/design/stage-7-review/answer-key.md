# Readability check: maintainer answer key

Do not read this until after answering the two packets. This is a scoring key,
not a record of human answers. Score each numbered answer as one point only
when all its parts are correct. Pass: at least 7/8 per packet, including 7 and 8.

## Gate

1. True when selected is true even if changed is false; false when both are false.
2. `ls` is a trusted reader, so false. An unlisted tool with both flags false
   may change files, so true. These are classifications, not execution effects.
3. A new Failed failure gives SendBack; an already-sent Failed failure gives NoProgress.
4. Passed gives Proved; TimedOut and NotStarted give Operator.
5. Preexisting Failed gives Operator. Preexisting Vacuous with a new failure
   still gives SendBack; the special preexisting case is specifically Failed.
6. `[Proved, Operator]` gives false; `[Proved, SendBack]` gives true.
7. Neither inference is valid. Gate functions classify supplied inputs. The
   enclosing folder is incomplete because `proof_outcome` has an open case
   and surviving mutants. This does not refute each gate function's own proof.
8. No. Determined establishes one allowed output outside declared freedoms;
   verified establishes the checked obligations and examples, not intent alignment.

## Cedar

1. The flags mean nonempty satisfied-forbid, satisfied-permit, residual-permit,
   and residual-forbid collections. All 16 combinations are admitted; residual
   policy evaluation depends on unknown information.
2. Deny: a satisfied forbid takes precedence.
3. Deny: an unresolved forbid does not supply a permit.
4. Unknown: the unresolved forbid may later override the permit.
5. Unknown: there is no satisfied permit yet.
6. Allow: more possible permits do not block a satisfied permit when no forbid applies.
7. No to both. Unknown is a determined authorization result; the obligations
   for this function proved and its open-case search settled.
8. Neither is proved. The booleans are supplied, not extracted by Accord from
   evaluated policies. Unknown is not an authorization grant. Application
   handling and the acceptability of the contract remain outside this proof.
