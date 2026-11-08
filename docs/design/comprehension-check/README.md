# Comprehension check

**Status:** registered on 2026-11-07, before any run. Not run yet. The stage 1
done-when in the [roadmap](../../../ROADMAP.md) and issue
[#2](https://github.com/sequelcore/accord/issues/2).

## Question

Do readers who have never seen Accord explain its code correctly without a
reference? It tests the first design goal in [influences](../influences.md)
on the [accepted syntax](../syntax.md).

## Material

Five snippets ([`snippets/`](snippets)), each with its own questions after a
`---` line. None of them is `proof-outcome-rule`, on which the syntax was
designed. Together they cover every construct of the accepted syntax:

| Snippet | Covers |
| --- | --- |
| S1 `late_fee` | Decision table with `otherwise`, `requires`, `intent`, labelled example |
| S2 `clamp` | `ensures`, `implies`, `result`, `if`/`else` as an expression |
| S3 `total` | Fixed-width integers in code and mathematical integers in specifications |
| S4 `next`, `after_two` | Enums, bare and qualified variants, a table without `otherwise`, a call without labels |
| S5 `grade` | First-match order: a row no input can reach |

## Arms

**Models.** The models of the
[probe](../probes/2026-11-07-intent-and-decision-tables/README.md):
`deepseek/deepseek-v4.1-flash`, `qwen/qwen3.8-omni-flash`,
`anthropic/claude-haiku-5.5`, `openai/gpt-6-luna`,
`mistralai/mistral-large-4-0`, `z-ai/glm-5.3-flashx` and `x-ai/grok-4.7`,
through OpenRouter, with [the prompt](prompt.txt), one snippet and its
questions as the only message: no system prompt, no tools. Each snippet goes
to each model twice, each time in a new conversation: 70 conversations. The
response records the exact model version. A reply that comes back empty is
run once more; if it is empty again, it is excluded and reported.

**People.** At least five developers who have not used Dafny, Verus or Lean
read the same snippets on screen, without a reference, and answer the same
questions in writing. Their time per snippet is recorded. This arm runs when
people are available.

## Scoring

Each answer is marked correct or incorrect by hand against the key below, in
`answers.csv` (`python score.py extract` writes the sheet). Every judgment
stays beside the quoted answer. `python score.py summarize` applies the pass
mark.

| Question | Correct when the answer says |
| --- | --- |
| S1.1 | `Flat` |
| S1.2 | `PerDay` |
| S1.3 | The call isn't allowed: it breaks `requires days_late >= 0`, so it is rejected or is an error. Returning a fee is incorrect |
| S1.4 | No: nothing checks it |
| S2.1 | `0` |
| S2.2 | The value the function returns |
| S2.3 | A value already inside the range comes back unchanged |
| S2.4 | Yes: `low` is always within the range, so the first `ensures` alone allows it |
| S3.1 | Without it, `a + b` could overflow `i32` (or exceed its maximum) |
| S3.2 | *Diagnostic.* No: arithmetic in specifications is mathematical |
| S3.3 | No: the sum exceeds 2147483647, which the second `requires` forbids |
| S4.1 | The three rows already cover every `Light` |
| S4.2 | `Yellow` would reach no row, so it is an error (rejected by the compiler, or no result). Returning a value is incorrect |
| S4.3 | They are the same value; `Light.Red` names the type, and inside `next` the type is already known. Saying the qualified form is required in the example is incorrect, since the parameter's type is known there too |
| S4.4 | The function has one parameter, so no label is needed |
| S5.1 | `Pass` |
| S5.2 | `Fail` |
| S5.3 | The `Honors` row can never be reached, because `score >= 60` matches first; so the function breaks its intent. Whether the language notices isn't scored |

**S3.2 is diagnostic.** Nothing in the snippet says that specifications use
mathematical integers, so readers aren't expected to know it. Its rate is
reported but doesn't count toward the pass mark; a low rate means the
reference and the receipt must state that rule where people read them.

## Pass mark

For each arm, over every scored question:

- at least **90%** of all answers correct;
- every question at least **70%** correct, so no construct is systematically
  misread;
- **S1.4** at least **90%** correct: `intent` must read as unchecked.

A question below 70% names a construct to redesign or document before stage 2
builds it. The pass mark doesn't change after a run.

## Result of the models arm (2026-11-07)

**Fail.** 70 conversations, all with a reply (one came back empty and was
run once more, as registered), cost $0.38. The hand judgments are in
[`answers.csv`](answers.csv) and the replies in [`replies.jsonl`](replies.jsonl).

| Measure | Result | Needed |
| --- | --- | --- |
| All scored answers | 92% | 90% |
| Weakest question | S4.4, 29% | 70% |
| S1.4, `intent` unchecked | 100% | 90% |

Every question scored 100% except these:

- **S4.3, 43%.** Eight of 14 replies said `Light.Red` is required in the
  example because there is no type there. In the accepted syntax the type is
  known there too, so this is a misreading the example invited: the reader
  can't see the rule for bare and qualified variants.
- **S4.4, 29%.** Most replies said calls in Accord are positional. S4 has no
  call with more than one parameter, so nothing in it shows that labels exist
  at all: the question can't be answered from its snippet. That is a defect
  of this registration. It is reported here, and the verdict stands.
- **S3.2, 71% (diagnostic).** Four replies assumed that `a + b` in a
  specification can overflow. The reference and the receipt have to state
  that specification arithmetic is mathematical.

By the pass mark, S4.3 and S4.4 name the constructs to redesign or document
before stage 2: how variants are qualified, and how labels show up in code
that has none.

## When stage 1 is done

**Accepted by the owner, 2026-11-07.** The models arm passing lets stage 2
start. The people arm runs when people are available, or with the feedback
of readers once the repository is public, and it can reopen any construct
it shows to be misread.
