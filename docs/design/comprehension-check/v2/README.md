# Comprehension check, version 2

**Status:** registered on 2026-11-07, before any run. Not run yet. It
follows [version 1](../README.md), which failed, and compares three ways of
writing enum variants instead of assuming one.

## Questions

1. Which way of writing variants do readers who have never seen Accord read
   correctly: bare (`Red`), always qualified (`Light.Red`), or a leading dot
   where the type is known (`.Red`)?
2. Does each form read correctly enough to pass the same mark as version 1?

## What changed from version 1

- **Three forms of every snippet**, generated from one template by
  [`forms.py`](forms.py), so the forms differ only in how variants are
  spelled ([`templates/`](templates), [`forms/`](forms)).
- **S4 has two enums that share a variant name** (`Light.Red` and
  `Alert.Red`), so a reader of the bare form has to work out which `Red` is
  which. That is the case where the forms should differ.
- **No question asks why the code is written as it is**; questions ask what
  the code does or means.
- **Rules a snippet can't show are diagnostic**, reported but not scored:
  whether labels are required (S1.5), whether specification arithmetic can
  overflow (S3.2), and whether the compiler reports an unreachable row (S5.4).
- **Questions every reader answered correctly in version 1 on the same code**
  are left out, except where they anchor a snippet.
- **Marking by rule where possible.** Value and yes/no questions are marked by
  [`score.py`](score.py); an answer naming more than one value is marked by
  hand, like the open questions.

## Arms

The models of version 1, through OpenRouter, with [the prompt](prompt.txt),
one snippet and its questions as the only message: no system prompt, no
tools. Each form of each snippet goes to each model twice, each time in a new
conversation: 3 forms × 5 snippets × 7 models × 2 = 210 conversations. A
reply that comes back empty is run once more; if it is empty again, it is
excluded and reported. The people arm of version 1 still applies, on the
form chosen here.

## Key

| Question | Correct when the answer says | Marked |
| --- | --- | --- |
| S1.1 | `Flat` | By rule |
| S1.2 | `PerDay` | By rule |
| S1.3 | The call isn't allowed: it breaks `requires days_late >= 0`. Naming a fee it returns is incorrect | By hand |
| S1.4 | No | By rule |
| S1.5 | *Diagnostic.* No: labels are required when a function has more than one parameter | By rule |
| S2.1 | `0` | By rule |
| S2.2 | Yes | By rule |
| S3.1 | No | By rule |
| S3.2 | *Diagnostic.* No: specification arithmetic is mathematical | By rule |
| S4.1 | `Clear` | By rule |
| S4.2 | The first `Red` is `Light`'s and the second is `Alert`'s. Both must be right | By hand |
| S4.3 | `Yellow` would reach no row, so the definition is rejected or has no result for it. Returning a value is incorrect | By hand |
| S4.4 | The three rows already cover every `Light` | By hand |
| S5.1 | `Pass` | By rule |
| S5.2 | `Fail` | By rule |
| S5.3 | The `Honors` row, because `score >= 60` decides first | By hand |
| S5.4 | *Diagnostic.* The language reports it | By hand |

Any spelling of a variant counts (`Flat`, `Fee.Flat`, `.Flat`).

## Pass mark and choice

For each form, over its scored questions:

- at least **90%** of all answers correct;
- every question at least **70%** correct;
- **S1.4** at least **90%** correct.

**Choice.** Among the forms that pass, the one with the most correct S4.2
answers. If two or more are within two answers of the best, the maintainers
choose among them. If no form passes, none is chosen, and the failing
questions name what to redesign. These rules don't change after a run.

## Result (2026-11-07)

**All three forms pass, and they tie on S4.2, so by the choice rule the
maintainers choose.** 210 conversations, all with a reply (two came back
empty and were run once more, as registered), cost $0.99. The judgments are
in [`answers.csv`](answers.csv) and the replies in
[`replies.jsonl`](replies.jsonl).

| Form | All scored answers | Weakest question | S1.4 | S4.2 |
| --- | --- | --- | --- | --- |
| short (`Red`) | 99% | S2.2, 93% | 100% | 14/14 |
| qualified (`Light.Red`) | 99% | S2.2, 93% | 100% | 14/14 |
| dot (`.Red`) | 99% | S2.2, 93% | 100% | 14/14 |

- **Every reader of every form told the two `Red`s apart** (S4.2), the case
  where the forms were expected to differ. On this evidence the three forms
  read equally well for these models.
- **The S2.2 key is wrong.** The three answers marked incorrect (GPT-6 Luna,
  dot; GLM 5.3 FlashX, qualified and short) say a body that always returns
  `low` breaks the function's `example`, which is also a promise. They are
  right; the key considered only the `ensures` lines. Version 1's S2.4 had
  the same flaw. The verdict stands either way: counted correct, S2.2 is
  100% in every form.
- **Diagnostics.** Specification arithmetic (S3.2) was read as mathematical
  in 4, 4 and 7 of 14 replies (dot, qualified, short); almost no reader
  expected the compiler to report an unreachable row (S5.4: 0, 1 and 1 of 14);
  most expected an unlabelled multi-parameter call to be rejected (S1.5: 14,
  12 and 13 of 14). The first two are rules the reference and the receipt
  have to state; the compiler has to show the second.

## Run

`OPENROUTER_API_KEY=... python run.py <models> 2`, then
`python score.py extract`, mark the empty `correct` cells in `answers.csv`
by hand, and `python score.py summarize`.
