# E2: human review

**Status: draft, not registered. No participant run or human outcome.**
The corpus, cards, allocation,
and scorer are concrete preparation, not experimental observations. Registration
and recruitment require a later decision. The separate
[readability check](../stage-7-review/README.md) must pass before E2 runs.

## Question and evidence

Do people find more true requirement defects per minute with Accord's review
view than with code alone or specifications alone, without more false positives
or misplaced confidence?

[Kaufman et al.](https://arxiv.org/html/2607.08885v1) found an assertion-judgment
gap among 86 Python programmers and no overall benefit from prose explanations.
That study does not establish an effect for Accord's review view.
[Gerdes et al.](https://research.chalmers.se/publication/510284/file/510284_Fulltext.pdf)
randomized 22 Erlang students between selected and random example suites and
found better API comprehension with selected examples. This is a different
outcome from finding defects. The
[Bosch study](https://link.springer.com/article/10.1007/s10664-023-10353-4)
evaluated counterexample explanation with a 13-person pretest–posttest study;
fixed order and different tasks limit causal attribution.

These sources were inspected for the stage 7 design. They motivate testing
examples, witnesses, and explicit limits together. They do not supply an
expected effect size, justify this sample size, or isolate the benefit of a
“not proved” list. E2 tests the bundle, not individual presentation features.

## Tasks and ground truth

The [corpus](corpus) has six variants in each of two domains: gate verdicts and
Cedar partial authorization. IDs R01–R12 hide categories on participant cards.
The [maintainer answer key](answer-key.json) records expected proof standing,
openness, a requirement defect, and a distinguishing input. Do not show the
key, corpus, other arms, or this page to participants. Anyone previously exposed
to them is ineligible. These files are public research material; that exposure
limit must be checked during recruitment.

| Category per domain | Candidate behavior | Expected check |
| --- | --- | --- |
| Clean | Reference rule with a complete contract | Verified; no intended defect |
| Body | Incorrect precedence or result under the correct contract | Failed; a postcondition is refuted |
| Weak specification | Remove two necessary implication directions; keep the correct body | All obligations prove, but an open case remains and mutants survive |
| Wrong complete | Code and complete specification agree on an incorrect result | Verified and determined; the requirement still disagrees |
| Excluded input | Requires excludes a case the requirement admits | Verified on the narrower domain |
| Misdeclared freedom | A declared freedom covers the weak contract's ambiguity | Verified; the freedom contradicts the requirement |

The weak-specification and freedom cases keep the correct body; the wrong
complete case changes both body and contract. A proof cannot decide whether
the independent requirement or the candidate specification is intended.
The selected example calls are fixed within each domain across all six variants.
The planted witness is held out of those examples. This prevents different
example counts from explaining the treatment effect, but may make these small
tasks easier or harder than production review.

`materials.py` checks every expected category, every declared example, the
upstream-derived reference result at the key's witness, and each planted fault.
For weak specifications and freedoms it also checks a constant alternative
under preconditions pinning the witness: the incorrect-by-requirement answer
must meet the candidate contract. For excluded inputs the added example must
fail; that does not prove the excluded input was outside the intended domain.
The independent requirement and answer key need owner review before registration.

Mutation analysis is omitted when the original obligations have not proved.
A repairing mutant of an invalid baseline is not evidence of a weak contract.

## Reviewers and allocation

Proposed sample: 90 adult developers, 30 per arm, each completing one gate and
one Cedar task. Use 45 with prior contract or formal-verification experience
and 45 with less experience, 15 of each per arm. This is an exploratory quota,
not a power calculation. Recruitment feasibility and interval precision must
be evaluated before the registration is frozen.

The [allocation](allocation.csv) has anonymous slots, not recruited people.
Seed 4607 randomizes the slots. Each variant appears five times per arm, with
15 gate-first and 15 Cedar-first orders per arm. Assign eligible people to
unused slots within their experience stratum, without inspecting answers.
Never show two versions of one rule to the same person. No Accord contributor,
material author, answer-key reader, or readability-packet reviewer is eligible.

Run a disjoint logistics pilot before registration. Publish its recruitment,
timing and comprehension problems, then freeze any changes and a precision
simulation before collecting E2 observations. No such pilot or simulation has
run. Do not change sample size after inspecting E2 effects.

## Arms and procedure

Every arm receives the same independently stated requirement and task prompt.

| Arm | Additional candidate material |
| --- | --- |
| Code | Signature, enums, and body; no contract, intent, examples, or evidence |
| Specification | Signature, enums, unchecked intent, requires, ensures, and freedoms; no body or results |
| Review | Actual terminal review view: intent, complete specification, examples, results, witnesses, freedoms, mutants, limits, and diagnostic codes; no complete body |

The [cards](cards) are generated from checked corpus files and actual compiler
output. Review cards remove only cache-use markers. The review arm has more
information; this compares useful bundles rather than isolating typography.
An ablation would be needed to attribute a benefit to a specific section.

Give everyone an unrelated practice task and the same short glossary. Allow
eight minutes per task, with no execution tools, assistants, source browsing,
or other participants. Capture active task time through submission or the
480-second limit, including unsuccessful work. Ask for defects, affected
promises, a concrete input and required result, and accept, request_change, or
insufficient_information. A reviewer may state not_observable. Collect confidence
0–100 after the timed task, with no feedback between tasks.

Some defects cannot be observed in every arm. Specification-only reviewers
cannot inspect a body-only error; code-only reviewers cannot inspect an omitted
promise when the body is correct. Report the practical comparison across all
tasks and the observable subsets separately. Wrong-complete tasks are the
common observable defective subset in this small corpus. Do not score an
unobservable body's acceptance as a calibration error.

## Measures and adjudication

- Primary: unique true predeclared requirement defects found per active review minute,
  calculated as total true finds divided by total active minutes per arm.
- False positives: invalid defect claims per task, including clean controls.
  Report precision, clean-control rejection, and false acceptance as well.
- Confidence: distributions for correct and incorrect dispositions, with
  unscored dispositions separate; high-confidence (at least 80) wrong acceptance
  rate among observable defective tasks. Confidence is not accuracy.
- Secondary: recall by domain and category, time to first valid finding,
  completion time, unfinished tasks, and requests for information.

Two people independently adjudicate anonymous raw claims against the requirement
and committed key, blind to arm when practical. A true find needs the matching
defect and a valid distinguishing or excluded-input case. Record disagreements,
their resolution, and additional valid defects outside the key. Replay witnesses
with Accord where expressible; do not let a verifier adjudicate human intent.
Deduplicate repeated true descriptions of the same defect. Invalid submitted
claims are false positives. The scorer consumes these adjudications; it does
not grade raw prose automatically.

Store observations outside this public repository under a consent and retention
policy selected before recruitment. An adjudicated input is a JSON array:

```json
[{"reviewer":"P001","arm":"review","stratum":"experienced","task":"R02",
  "seconds":120,"finished":true,"first_valid_seconds":90,
  "disposition":"request_change","confidence":80,
  "claims":[{"defect":"gate_body","valid":true,"witness_valid":true}]}]
```

This is an illustrative shape, not a real observation or a claim about P001's
allocated task. The CLI rejects observations inconsistent with allocation,
duplicate reviewer/task records, invalid times or confidence, and unadjudicated
claims. Empty claims are valid. Keep raw claims, adjudication, and timestamps
with the external study data so the summary can be audited.
Here valid means an independently adjudicated match to the predeclared defect
and requirement. Keep independently valid findings outside the frozen key in
a separate analysis, outside both primary true counts and false-positive counts.
`first_valid_seconds` is nullable when unavailable or when no valid find exists;
it must never be invented from the final elapsed time.

## Draft analysis and pass mark

Resample whole reviewers within arm and experience stratum, retaining their
two tasks together: 10,000 bootstrap draws, seed 4607. Use two-sided 97.5%
intervals for each of the two review-minus-baseline rate differences, targeting
95% simultaneous coverage by Bonferroni adjustment. Apply the same adjustment
to the pair of false-positive differences. Coverage is nominal and approximate,
especially with five reviews per variant per arm. Show domain/category results;
do not treat repeated tasks as independent people.

The proposed product pass mark requires all of these:

1. At least 25% higher observed true-finds rate than each baseline, and each
   adjusted rate-difference interval has a lower bound above zero.
2. Each adjusted upper bound for excess false positives is at most 0.10 per task.
3. Both domains have a positive observed rate difference against both baselines.
4. Recall on the common observable defective subset does not fall below either
   baseline, and observed high-confidence wrong-acceptance rate does not rise.

A zero baseline makes the relative ratio unavailable, not infinite success.
Missing contrasts, insufficient observations, or an undefined ratio give no
pass verdict. A nonpassing result is not proof of no benefit; wide intervals
can be inconclusive. Calibration guardrails are observed checks, not a claim of
statistically established calibration improvement. The scorer deliberately labels
all output as analysis under an unregistered draft.

Exclude only absent consent/eligibility, duplicate participation, and documented
technical interruption that makes timing unusable. Retain available accuracy
data from interrupted tasks in a separately disclosed analysis. Do not exclude
wrong, slow, unfinished, or low-confidence answers. No post hoc speed cutoff.
Stop at the frozen sample or a recruitment deadline fixed before registration;
report any shortfall and incomplete design cells. The current scorer withholds
the proposed pass verdict for a shortfall.

Before registration: resolve human readability, independent key review, recruitment
feasibility, pilot, precision analysis, deadline, consent, data retention, and
missing-data handling. Freeze the protocol, all hashes, binary/source revision,
solver versions, allocation and scorer in a timestamped registration before the
first E2 participant. This draft is not that registration.

## Reproduce the material checks

With Python 3.10 or later and the pinned solvers configured:

```sh
cargo build --locked
python docs/design/e2-human-review/materials.py target/debug/accord --write
python docs/design/e2-human-review/materials.py target/debug/accord
python docs/design/e2-human-review/score.py --self-test
```

On Windows use `target/debug/accord.exe`. `--write` deliberately regenerates
cards, allocation, packets, and hashes. Without it, stale generated material
fails. Do not hand-edit generated files. The corpus, requirements in
`materials.py`, key, and protocol are the authored inputs. Solver witness choices
can differ between solver builds; keep versions pinned, inspect differences,
and never replace an expected semantic category just to pass validation.
Scorer self-tests use synthetic observations only. No participant result file
is checked into this draft.
