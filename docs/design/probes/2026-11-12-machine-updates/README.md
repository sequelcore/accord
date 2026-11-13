# Probe: how a machine's updates are written (2026-11-12)

An informal probe to guide stage 10's syntax for updates in `machine`
events ([#82](https://github.com/sequelcore/accord/issues/82)). Its
questions, answer key and decision rule were written before any run. It is
not a registered experiment: it has no pass mark, and its result guides the
design without counting as evidence that the syntax is understood.

## Question

Do readers who have never seen Accord predict a machine's next state
correctly, and which reading does each way of writing an update suggest:
that a later expression in the same event sees the old value, or the new
one?

## Method

Tesota's ProviderRetry model as an Accord machine, written in three versions
([`variants/`](variants)), identical except for how an event states the next
value of a field:

| Version | Form |
| --- | --- |
| A | `phase = Waiting`, one update per line |
| B | `next { phase: Waiting, retries: retries + 1 }` |
| C | `phase' = Waiting`, the primed form of TLA+ and Quint |

In all three a field an event doesn't name keeps its value, and an `init`
lists every field (as `phase = Failed` in A and C, as a field list in B).
Question 3 gives a rewrite of `schedule` in each version's own notation
([`rewrite-A.txt`](rewrite-A.txt), [`rewrite-B.txt`](rewrite-B.txt),
[`rewrite-C.txt`](rewrite-C.txt)) that reads `retries` after setting it.

Each version goes to each reader once, in a new conversation, with
[the prompt](prompt.txt) as the only message, its `{code}` filled with the
version and its `{rewrite}` with that version's rewrite. Readers: Claude models
(Haiku, Sonnet, Opus, Fable) in fresh contexts with no repository access,
and, if run, the five public chat models stage 9 used (ChatGPT, Grok,
DeepSeek, Qwen, Gemini).

## Answer key

1. `phase` Calling, `calls` 2, `retries` 1, `time` 1, `last_start` 1,
   `last_due` 1.
2. `phase` Ended, `due` 0, `retries` 0: `0 + 121 < 120` is false, and the
   fields the `else` branch doesn't name keep their values.
3. In every version, every expression in an event sees the state before
   the event, so the condition reads `retries == 2` and the phase is
   **Waiting**. In version A, Accord rejects this rewrite, since it reads a
   field the event already set. The reply is scored by the reading it
   commits to: Ended (the update already took effect), Waiting (the old
   value), or the ambiguity pointed out.
4. `phase` (to Failed) and `terminal` (to true); every other field keeps
   its value.
5. Both must hold in every reachable state; helper invariants exist so the
   other invariants can be proved, and aren't claims of their own.
6. No: `schedule` returns the next state and leaves `state` unchanged.

Questions 7 and 8 are recorded, not scored.

## What would change the decision

Written before the runs:

- A is replaced by C's labelled spelling, or by B, if A is answered wrongly
  on question 1, 2 or 4 where B or C is answered rightly, or if A's replies
  say on question 6 that `state` changes where B's and C's don't.
- If A's replies answer question 3 with Ended where B's or C's answer
  Waiting, the rule and its diagnostic are required for A; that alone
  doesn't replace A.
- `helper invariant` is renamed if question 5 is missed.
- If every version is answered at the ceiling, A is kept, as stage 9 kept
  its shorter form.

## Result

Twenty-four replies ([`replies.md`](replies.md)): Haiku, Sonnet, Opus and
Fable through fresh agents told to use no tools, and ChatGPT, Grok, DeepSeek
and Gemini through their public chat interfaces, each in a conversation with
no memory of earlier ones. Qwen wasn't run: the browser extension had no
permission on its site.

Questions 1, 2, 4 and 5 were answered as the key gives in all 24 replies,
helper invariants by inference ("scaffolding for the proof"). On question 6,
23 replies said `state` doesn't change; ChatGPT, on A, said it can't be
determined. Confidence was 4 or 5 in every reply that gave one.

Question 3:

| Reader | A | B | C |
| --- | --- | --- | --- |
| Haiku | Ended | Waiting | Waiting |
| Sonnet | Ended | Waiting, after first writing Ended | Waiting |
| Opus | Ended | Waiting | Waiting |
| Fable | Ended | Waiting | Waiting |
| ChatGPT | Ended | Waiting | Waiting |
| Grok | Waiting, reasoned from the value after the increment | Waiting | Ended |
| DeepSeek | Ended | Waiting | Waiting |
| Gemini | Ended | Waiting | Waiting |

- **A is read sequentially.** Seven replies answered Ended, and the eighth
  reasoned from the incremented value before misapplying `>=`.
- **B is read as the state before the event** in all eight. Several replies
  named the rule they assumed: "every expression in `next` reads the state
  from before the event."
- **C is read as the old value** in seven of eight; Grok read the prime
  sequentially.
- **What was hardest**, across versions: what `helper` means, what happens
  when a `requires` fails, and in B and C whether the new or the old state is
  read.

## Decision

The rule written before the runs keeps A, with the rejection of a read after
a set and its diagnostic required. The maintainers chose B instead: it was
the only form that every reader read as Accord means it, and with A every
author, person or agent, would write order-dependent updates by instinct and
meet the diagnostic. B's costs are that `next` is a reserved word, so no
parameter or field can use it, and that `next { field: value }` looks like a
record before records exist; stage 11 decides records on their own.
`helper invariant` is kept. Accepted as
[decision 7](../../syntax.md#decisions).

## Limits

- One run per reader and version; the public interfaces' model versions and
  settings weren't recorded, so the run can't be repeated exactly.
- Grok searched the web on C ("9 fuentes") despite the instruction, and
  DeepSeek showed a reasoning step with its "deep thinking" option apparently
  off.
- The prompt stated none of the semantics, on purpose, so it measures a
  first reading, not a reading after the reference. No person was asked.
