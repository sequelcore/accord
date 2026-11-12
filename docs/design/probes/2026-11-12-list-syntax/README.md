# Probe: the syntax of quantifiers over lists (2026-11-12)

An informal probe to guide stage 9's syntax for `all`, `any`, `count`,
length and indexing ([#69](https://github.com/sequelcore/accord/issues/69)).
Its questions and answer key were written before the runs, but it is not a
registered experiment: it has no pass mark, and its result guides the design
without counting as evidence that the syntax is understood.

## Question

Do readers who have never seen Accord evaluate quantifiers over lists
correctly in the proposed form, by element (`all x in items: …`) and by
index (`all k in indices(items): …`), compared with the bounded-index form
of Dafny and Verus (`forall k: 0 <= k and k < length(items) implies …`)?

## Method

Four functions shaped like Tesota's `verdict-rule` (`decisions`,
`partly_holds`, `path_covered` and a covered-paths check over three
parallel lists), written in two versions ([`variants/`](variants)):

| Version | Form |
| --- | --- |
| A | `all`, `any` and `count` by element, and `all k in indices(items)` by index |
| B | `forall`, `exists` and `count` over a bounded index, as in Dafny and Verus |

Both use `length(items)` and `items[k]`. Each version went to five models in
their public chat interfaces, once, in a new conversation each time, with
[the prompt](prompt.txt) and the code as the only message: ChatGPT, Grok,
DeepSeek, Qwen and Gemini, with web search off where the interface allowed
it. The model versions behind each interface weren't recorded.
[`replies.md`](replies.md) holds the ten replies.

## Answer key

1. `2`.
2. `false`.
3. `false`: index 1 is neither proved nor exercised.
4. `true`: a quantifier over no index holds.
5. `k` is an index into the lists, taking each value from 0 to the length
   minus one, and none for empty lists.

## Result

| Model | A | B |
| --- | --- | --- |
| ChatGPT | 5 of 5, confidence 5 | 5 of 5, confidence 5 |
| Grok | 5 of 5, confidence 5 | 5 of 5, confidence 5 |
| DeepSeek | 5 of 5, confidence 5 | 5 of 5, confidence 5 |
| Qwen | 5 of 5, confidence 5 | 5 of 5, confidence 5 |
| Gemini | 5 of 5, confidence 5 | 5 of 5, confidence 5 |

- **Every reply was right on every question,** the empty lists and the
  parallel index included, so the two forms didn't differ.
- **What was hardest:** in A, the quantifier syntax and the labelled
  arguments; in B, how dense `forall … implies …` is. One reply to B found
  nothing hard because it resembles Dafny and Boogie.
- **Consequence:** the proposed form, A, is kept: it is shorter and reads
  without bounds written by hand, and the probe gives no reason against it.

## Limits

- The task hit a ceiling: four small functions. It can't separate the forms
  for these models, and says nothing about people, who were not asked.
- Models know Dafny and Verus, whose notation B uses.
- One run per model and version, through chat interfaces whose model
  versions and settings weren't recorded, so it can't be rerun exactly.
- Scored by reading every reply against the key.
