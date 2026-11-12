# Stage 7 human readability check

**Status: moved to the public phase; no human answers scored yet.**
This is an engineering acceptance check, separate from
[E2](../e2-human-review/README.md), which remains draft and unregistered.
Automated tests establish output contents, evidence routing and material
consistency; they do not establish human readability.

Read the [gate packet](gate-packet.md) and [Cedar packet](cedar-packet.md), without
opening source or the answer key. Return answers 1–8 for each and identify
anything unclear. The gate packet includes all five functions in `gate-rule.accord`,
plus the enclosing program's incomplete standing. The Cedar packet explains
the four Boolean inputs and distinguishes domain Unknown from solver uncertainty.

A reader from outside the project performs this check in the public phase,
before E2 ([decision](../../../ROADMAP.md#decisions)): someone who wrote or
maintains Accord can't show that the review reads without its code. The
criterion is at least 7/8 on each packet, with the final two scope questions correct on each. Those questions
protect against interpreting a function proof as proof of intent, upstream
inputs, surrounding systems, or an unresolved authorization decision. Score
against the [maintainer key](answer-key.md) only after answering; record answers,
packet hashes, score, and unclear wording before claiming the gate passed.
This one-person acceptance check cannot establish general usability or an E2
effect. Its reviewer is ineligible for E2. This repository is public: record the
answers and score here without the reader's name or anything that identifies
them.

The packets are generated from actual `accord check` output by
`../e2-human-review/materials.py`; only cache-use markers are removed. Their
hashes are in `../e2-human-review/material-hashes.json`. Rebuild Accord and run
that script without `--write` to check freshness. If source, wording, or evidence
changes after review, compare review hashes and repeat affected questions.
No approval storage or automatic acceptance is implemented in the compiler.
