# Replies

Twelve replies, condensed: each reader's answer to each question, keeping its
values, its result and the reading it committed to. Claude Haiku, Sonnet,
Opus and Fable, each in a fresh agent told to read only the prompt file and
use no other tool; each used one read. Two Fable runs ended on a server
overload (HTTP 529) before answering and were rerun from scratch; A needed
two reruns.

## Version A

**Haiku.** 1: `Version { … pre: [Alpha(text: "alpha"), Numeric(digits: "1")], build: [] }` and the same with `Alpha(text: "beta")`; -1. 2: -1. 3: 1. 4: `s` and `t`, the texts of x's and y's `Alpha`. 5: not stated; most likely rejected as non-exhaustive, else falls through. 6: termination measure, checked by the verifier at proof time. 7: not called by executable code; export not stated, can't answer. 8: Q5 2, Q7 export 1, others 3 to 5. 9: the `ensures` of `compare_lists`, the swapped `lower_at(a: b, b: a, …)`, unused names in `id_lower`.

**Sonnet.** 1: values in A's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: expects a compile error; can't tell whether a warning or a runtime failure. 6: termination measure; the compiler or verifier checks it. 7: never called at run time; assumes erased and not exported. 8: 4. 9: the `ensures` of `compare_lists`; whether spec code is erased; whether a non-exhaustive match is an error.

**Opus.** 1: values in A's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: expects the compiler to reject it (inferred). 6: termination measure, proved statically. 7: never called at run time; expects it erased and not exported. 8: 5 for 1 to 3, 3 to 4 for 5 to 7. 9: the `spec fn`/`fn` split and what is erased; the swapped arguments; the order of `<` on `Text`.

**Fable.** 1: values in A's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: rejected as non-exhaustive, no TypeScript produced (inferred). 6: termination measure, checked statically. 7: no and no, by inference. 8: 4. 9: the `ensures` of `compare_lists`; `d < e` on digits relies on equal lengths and no leading zeros.

## Version B

**Haiku.** 1: `Version { … pre: [Alpha { text: "alpha" }, Numeric { digits: "1" }], build: [] }` and the same with `Alpha { text: "beta" }`; -1. 2: -1. 3: 1. 4: `text` and `other`. 5: a checker should reject it (inferred). 6: termination measure, checked by the verifier. 7: never called by executable code; export unknown, infers not exported. 8: 4; export 2. 9: the `ensures` of `compare_lists` and the swapped arguments.

**Sonnet.** 1: values in B's spelling; -1. 2: -1. 3: 1. 4: `text` and `other`. 5: most likely a non-exhaustive match error; if not, falls through at run time. 6: termination measure, static. 7: probably never called; expects it erased and not exported. 8: 4. 9: the `ensures` with spec functions; `!=` on enums assumed structural; `<` on `Text` undefined.

**Opus.** 1: values in B's spelling; -1. 2: -1. 3: 1. 4: `text` and `other`. 5: expects a compile or verification error, not a runtime fallthrough. 6: termination measure, static. 7: never called; erased, not exported (inferred). 8: 4. 9: what `spec fn` means for output; precedence of `implies` and `and`; the swapped arguments.

**Fable.** 1: values in B's spelling; -1. 2: -1. 3: 1. 4: `text` and `other`. 5: rejected, nothing emitted (inferred). 6: termination measure, static. 7: no and no, as read. 8: 4. 9: the `ensures` of `compare_lists`; inner patterns that rebind a name already bound outside and go unused; `<` on `Text` and `!=` on `PreId` unstated.

## Version C

**Haiku.** 1: `Version(major: 1, minor: 0, patch: 0, pre: [Alpha(text: "alpha"), Numeric(digits: "1")], build: [])` and the same with `Alpha(text: "beta")`; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: should be rejected at compile time in a verified language; not certain. 6: termination measure, at proof time. 7: never called on a runtime path; export unknown from the file. 8: export 1, others 4 to 5. 9: the `ensures` of `compare_lists` and the swapped call.

**Sonnet.** 1: values in C's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: presumably rejected, or a failure at run time if allowed; doesn't know which. 6: termination measure, checked statically. 7: `function` marks a specification function, so never called at run time and not exported (inferred). 8: 4 for 1 to 4, 3 for 5 to 7. 9: `function` against `fn`; `!=` on enums assumed structural.

**Opus.** 1: values in C's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: expects the program rejected (inferred). 6: termination measure, at compile time. 7: no to both: `function` looks like ghost code, erased. 8: 4. 9: `function` against `fn`, never stated; the order of `<` on `Text`.

**Fable.** 1: values in C's spelling; -1. 2: -1. 3: 1. 4: `s` and `t`. 5: rejected as non-exhaustive (inferred). 6: termination measure, static. 7: never called; `function` read as specification only and erased, so not exported (inferred). 8: 4. 9: `function` against `fn`, unexplained; the swapped arguments; `<` on `Text`.
