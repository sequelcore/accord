# Probe: intent and decision tables (2026-11-07)

An informal probe to guide the stage 1 syntax design. Its questions and
answer key were written before the runs, but it is not the registered
comprehension check: it has no pass mark, and its results guide the design
without counting as evidence that the syntax is understood.

## Questions

1. Do readers who have never seen Accord take `intent` as unchecked?
2. Do they notice when code breaks its intent, while its contract and body
   agree with each other?
3. Does a contract written in symbols (`<==>`, `==>`) or a decision table
   (`decide`) make that easier?

## Method

The rule is Tesota's `proof-outcome-rule`, written in four versions
([`variants/`](variants)):

| Version | Form | The code |
| --- | --- | --- |
| A1 | Contract and body | Breaks the intent; contract and body agree |
| A2 | Contract and body | Correct |
| C1 | Decision table | Breaks the intent |
| C2 | Decision table | Correct |

The planted error: a run that timed out but verified something returns
`Passed`, although the intent says a proof passes only when nothing stopped
it. In A1 the contract omits `timed_out` for `Passed`, so a compiler would
prove the function; only the intent reveals the error.

Each version went to seven models through OpenRouter, twice, in a new
conversation each time, with [the prompt](prompt.txt) and the code as the
only message: no system prompt, no tools. The models were DeepSeek v4.1
Flash, Qwen 3.8 Omni Flash, Claude Haiku 5.5, GPT-6 Luna, Mistral Large 4,
GLM 5.3 FlashX and Grok 4.7. Eight replies that hit the 4,000-token limit
empty were run again with 16,000. [`replies.jsonl`](replies.jsonl) holds the
56 replies kept; they cost $0.40, and the discarded attempts about $0.06.

To run it again: `OPENROUTER_API_KEY=... python run.py <models> <repetitions>`,
then `python score.py`. Coding agents weren't used: their system prompts and
instruction files carry context that would bias the answers.

## Answer key

- A1 and C1: question 2 is **no**, and question 3's input has `timed_out`
  true, `cancelled` false, `started` true, `regenerated` true, `check_exit` 0
  and `verified` above 0, returning `Passed`. A "no" with another input
  doesn't count as a detection.
- A2 and C2: question 2 is **yes**. Any "no" is a false alarm.

## Results

| Version | Result |
| --- | --- |
| A1 | 14 of 14 detected |
| A2 | 14 of 14 correct |
| C1 | 14 of 14 detected |
| C2 | 13 of 14 correct, 1 false alarm |

- **`intent` reads as unchecked.** By pattern, 45 of 56 replies say that
  nothing checks it; of the other 11, the 4 read by hand say the same, and 7
  weren't read. In an earlier probe by hand, the same rule with `claim`
  instead of `intent`, three models (ChatGPT, Grok, DeepSeek) and nine
  conversations, only 3 of 9 replies took `claim` as unchecked.
- **The error was found every time, in both forms.** Confidence was mostly
  4 or 5 out of 5.
- **The two forms didn't differ** for these models.
- **The intent's wording is ambiguous.** The false alarm (Qwen, C2) read
  "passes only when …" as a sufficient condition rather than a necessary one,
  and Claude Haiku and GLM raised the same ambiguity in correct replies.

## Limits

- The task hit a ceiling: one bug in a six-row rule. It can't separate the
  forms for these models, and says nothing about people, who were not asked.
- Models know Dafny and Verus, whose notation A uses; developers who don't
  may read A very differently.
- Scoring is by pattern, with spot reading; the 56 replies weren't all read
  in full.

## What it changed

- The keyword for intent is `intent`, not `claim`.
- Decision rules are written as decision tables; `requires` and `ensures`
  remain for everything else. This is a design choice the probe allows, not
  one it shows to be better.
- An intent's own wording can be ambiguous, which the review view and the
  comprehension check have to cover.
