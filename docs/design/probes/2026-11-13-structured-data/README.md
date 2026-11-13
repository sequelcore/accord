# Probe: how records, sums, `match` and spec functions are written (2026-11-13)

An informal probe to guide stage 11's syntax
([#83](https://github.com/sequelcore/accord/issues/83), decision 7 of the
first set). Its questions, answer key and decision rule were written before
any run. It is not a registered experiment: it has no pass mark, and its
result guides the design without counting as evidence that the syntax is
understood.

## Question

Do readers who have never seen Accord read a record, a sum with fields, a
`match` and a spec function as Accord means them, and which spelling of each
reads best?

## Method

The SemVer comparator stage 11 ports, written in Accord in three versions
([`variants/`](variants)) that differ only in these spellings:

| Version | Variant with fields | Pattern | Record value | Spec function |
| --- | --- | --- | --- | --- |
| A | `Numeric(digits: Text)`, built as `Numeric(digits: "2")` | positional, `Numeric(d)` | `Version { major: 1, … }` | `spec fn` |
| B | `Numeric { digits: Text }`, built as `Numeric { digits: "2" }` | by field name, `Numeric { digits }`, renamed as `Numeric { digits: other }` | `Version { major: 1, … }` | `spec fn` |
| C | as A | as A | `Version(major: 1, …)`, like a labelled call | `function` |

A against B compares the patterns; A against C, the record value and the
spec function's keyword. Each version has spec functions that call each other
(`first_difference`, `id_lower`, `lower_at`), a recursive one with
`decreases`, and a contract stated through them.

Each version goes to each reader once, in a new conversation, with
[the prompt](prompt.txt) as the only message, its `{code}` filled with the
version and its `{deleted}` with that version's line `Alpha(t) => -1,` (A, C)
or `Alpha { text } => -1,` (B). Readers: Claude models (Haiku, Sonnet, Opus,
Fable) in fresh contexts with no tools and no repository access, and, if
run, the public chat models stage 10 used (ChatGPT, Grok, DeepSeek, Gemini).

## Answer key

1. In the version's own spelling, `Version { major: 1, minor: 0, patch: 0,
   pre: [Alpha(text: "alpha"), Numeric(digits: "1")], build: [] }` and the
   same with `Alpha(text: "beta")` (A); braces for the identifiers (B); the
   call form for the record (C). The result is **-1**: the lists first differ
   at index 1, and a number is below a word. The value is scored as written in
   the version's spelling, another spelling, or not given.
2. **-1**: a version with a pre-release is below the same version without
   one.
3. **1**: digits compare by length first, so 10 is above 9.
4. The text of x and the text of y: `s` and `t` (A, C), `text` and `other`
   (B).
5. Accord rejects the program: the `match` on y doesn't cover `Alpha`, and
   the error names it. Scored by the reading the reply commits to: rejected
   when compiling, a failure or other value when running, or unsure.
6. A measure that gets smaller at each recursive call and stays at least 0,
   so the recursion ends; the compiler proves it.
7. Accord means no and no: a spec function exists for contracts and proofs;
   it isn't run and isn't exported. Scored by the reading: not run, run, or
   unsure; and not exported, exported, or unsure.

Questions 8 and 9 are recorded, not scored.

## What would change the decision

Written before the runs:

- **Patterns.** B replaces A if a reader answers question 1, 3 or 4 wrongly
  on A and rightly on B more often than the reverse. If both are at the
  ceiling, A is kept: it is shorter, and its values are built like labelled
  calls, which Accord already has.
- **Record values.** C's call form replaces braces if more replies on C than
  on A write question 1's values in their version's own spelling. A tie keeps
  braces, which `next { field: value }` already uses.
- **Spec functions.** `function` replaces `spec fn` if more replies on C than
  on A read question 7 as "not run". If most replies on both read it as run,
  the keyword alone doesn't carry it: `spec fn` is kept and the reference and
  the diagnostics must say it.
- **Exhaustiveness.** If most replies read question 5 as a failure when
  running, the error must name the missing variant and the reference must
  state the rule; the syntax doesn't change.
- `decreases` is renamed if most replies miss question 6.

## Result

Twelve replies ([`replies.md`](replies.md)): Haiku, Sonnet, Opus and Fable,
each on each version, in fresh agents. The public chat models weren't run.

| Question | A | B | C |
| --- | --- | --- | --- |
| 1, values in the version's own spelling | 4 of 4 | 4 of 4 | 4 of 4 |
| 1, result -1 | 4 of 4 | 4 of 4 | 4 of 4 |
| 2, 3, 4, 6 as the key | 4 of 4 | 4 of 4 | 4 of 4 |
| 5, rejected when compiling, as the likeliest reading | 4 of 4; Haiku and Sonnet unsure | 4 of 4; Sonnet allows a runtime fallthrough | 4 of 4; Haiku and Sonnet unsure |
| 7, not run | 4 of 4 | 4 of 4 | 4 of 4 |
| 7, not exported | 3 of 4; Haiku can't tell | 4 of 4, Haiku by inference | 3 of 4; Haiku can't tell |

- **Every spelling was read as Accord means it.** Each reader built values in
  the spelling it was shown, from the one `example`, including C's call form.
- **A spec function was read as not run by all twelve,** under `spec fn` and
  under `function` alike, each time by inference: "the code does not state
  it". Whether it is exported was the least certain answer (confidence 1 or 2
  from Haiku on A and C).
- **Exhaustiveness was read as a compile-time rejection** by every reply that
  committed, always as an inference.
- **What was hardest**, across versions: the `ensures` of `compare_lists`,
  whose `result == 1` clause swaps the arguments of `lower_at` but not of
  `first_difference` (10 replies); whether spec code is erased, or what
  `function` means against `fn` (6); what `<` means on `Text` (5); whether
  `!=` on a sum is structural (3). On B, two
  readers noted names bound by an inner pattern that go unused.

## Decision

The rule written before the runs gives, with every version at the ceiling:
A's positional patterns and `Numeric(digits: "2")` values, records built with
braces, `spec fn`, exhaustiveness as a compile-time error, and `decreases`.
None of the replies distinguish the versions, so the rule's tie-breaks decide.

An independent review then found what the probe couldn't show. Every variant
here has one field, so no reader could swap fields by position; `match` arms
used a row syntax unlike `decide`'s; and a record built with braces must stay
apart from a block, as in `if x == On { On }`. The maintainers accepted the
rule's result with the review's changes, as
[syntax decisions 8 to 12](../../syntax.md#decisions): positional patterns
only for one field and labels beyond, `match` rows written as `decide`'s,
`_`, records with every field named, `spec fn` with mathematical integers
and no `requires`, and code point `length` and `<` on `Text`. The record's
braces and the row syntax are taste: the probe didn't measure them.

What the replies ask of the reference and the diagnostics, whatever is chosen:
state that a spec function isn't run or exported, that a `match` must cover
every variant, that `<` on `Text` is code point order, and that `==` on a
record or a sum compares fieldwise.

## Limits

- One run per reader and version, Claude models only; four readers per
  version, against eight in stage 10.
- Each reader read the prompt from a file outside the repository, told to use
  no other tool; each used one read. In stage 10 the prompt was the message.
- The variants were written by the agent that designed the probe; the
  comparator's contract shape came from the measurements, not from the
  readers.
- The prompt stated none of the semantics, on purpose, so it measures a first
  reading, not a reading after the reference. No person was asked.
