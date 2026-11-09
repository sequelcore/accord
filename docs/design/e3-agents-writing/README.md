# E3. Agents writing Accord

**Status:** proposed registration. It is registered when this page merges,
before any counted run; the question, tasks, success rule and pass mark don't
change after that. The [roadmap](../../../ROADMAP.md) lists it.

## Question

Given the same rules, is an agent's verified success with Accord comparable
to its verified success with TypeScript and
[LemmaScript](https://github.com/midspiral/LemmaScript)?

## Material

Ten rules, each written once in prose, in [`tasks.json`](tasks.json):

| Task | Rule | Source |
| --- | --- | --- |
| `t01-clamp` | limit a value to a range | written for E3 |
| `t02-sign` | the sign of an integer | written for E3 |
| `t03-largest` | the largest of three integers | written for E3 |
| `t04-within-budget` | either of two measurements within a limit | Tesota's `budget-rule` |
| `t05-grade` | a score's letter grade | written for E3 |
| `t06-distance` | the distance between two bounded integers | written for E3 |
| `t07-proof-outcome` | how a proof run ended, from six inputs | Tesota's `proof-outcome-rule` |
| `t08-mutant-finding` | what a mutant's proof outcome means | Tesota's `mutation-rule` |
| `t09-partial-decision` | an authorization decision with unevaluated policies | Cedar's `PartialResponse::decision` |
| `t10-gate-verdict` | what a verification gate does with a proof | Tesota's `gate-rule` |

Every rule uses only what both languages express without workarounds:
booleans, integers, enums, text equality and a list of texts with
membership; no division, no loops required in Accord. `t10` needs a search of
`sent`: a loop with invariants in LemmaScript, `in` in Accord. That
difference is part of what E3 measures.

For each task, an agent sees a task page, generated from `tasks.json` by
[`e3.py`](e3.py), with the prose, the function's exact signature in its
language and the type declarations to include; parameters and values are
named as its language names them (`timed_out` and `TimedOut` in Accord,
`timedOut` and `"timed_out"` in TypeScript). It also sees one reference page,
[`reference/accord.md`](reference/accord.md) or
[`reference/lemmascript.md`](reference/lemmascript.md): the same structure,
549 and 497 words, neither using a task as its example. The
[prompt](prompt.txt) is the same in both arms apart from the language's name,
the file's name and the check command.

**Hidden examples.** 81 input–result pairs, 6 to 16 per task, in
`tasks.json`, generated from Python references and never shown to an agent.
They catch a function that is proved against a contract weaker than the rule.

**Reference solutions.** [`solutions/`](solutions) solves every task in both
languages. `python score.py --self-test` checks that each is accepted by its
checker and passes all its hidden examples, and that a wrong solution fails
some, in each arm. It passes.

## Arms

- **Accord:** the file `answer.accord`, checked by `accord check`.
- **LemmaScript:** the file `answer.ts`, translated by `lsc gen` and proved by
  `lsc check --backend=dafny`, from LemmaScript 0.6.5 with the patch
  [Tesota](https://github.com/sequelcore/tesota) applies to it, and Dafny
  4.11. The generated `.dfy` is deleted before each check, so every check
  starts clean.

In both, the agent runs [`check.py`](check.py), which prints its checker's
own output and ends with `Result: accepted` or `Result: not accepted`, by the
same rule the scorer uses:

- **Accord:** the task's function is present, every obligation is `proved`
  and every example the agent wrote passes. An open case doesn't block
  acceptance, since LemmaScript has no counterpart; it is reported as a
  secondary measure.
- **LemmaScript:** the task's function is exported, and Dafny reports at least
  one verified obligation and no errors.

## Agents

Two models, Claude Haiku 5.5 and Claude Sonnet 5.5 (`--model haiku` and
`--model sonnet`; the run records the model each alias resolved to), through
Claude Code 2.1.319 in print mode, with a subscription's sign-in. Each run
gets a new, empty folder holding only `task.md` and `reference.md`, and runs
[`run.py`](run.py)'s command:

- `--restricted --strict-mcp-config`: no user or project instructions,
  memory, settings or MCP servers, and file tools confined to the folder;
- `--tools Read,Write,Edit,Bash`, with Bash allowed only for the check
  command.

Before registration, that isolation was tested: a run without these flags
reported a canary word from a `CLAUDE.md` and the operator's instruction
files, and the same run with them reported neither; a `cat` of a reference
solution from inside a run was denied.

Each agent has **five checks**, enforced by `check.py`, and twenty minutes.
Ten tasks × two arms × two models × two repetitions: **80 runs**, 40 per
arm, paired by task, model and repetition. Runs are resumable.

## Scoring

[`score.py`](score.py) rechecks each run's final file from a clean copy and
runs its hidden examples: Accord's through `example` lines appended to a
copy, LemmaScript's through Node. A run is a **verified success** if its
final file is accepted **and** passes every hidden example. A run with no
final file, or that ran out of checks or time, is not a success.

A run is **invalid** only when Claude Code ends it with an error before the
agent's first tool call, as a usage limit does. `run.py` runs an invalid run
again, up to three attempts in all; one still invalid after that counts as
not a success. Invalid attempts are reported. A run whose transcript records a permission
denial is scored and listed with the denied command, which tells an attempt
to leave the folder from a command the rules merely didn't allow, such as
`cd` chained before a read in the folder.

## Measures and pass mark

- **Verified success rate** per arm: successes over 40, overall, per model
  and per task.
- **Paired difference:** Accord's rate minus LemmaScript's over the 40 pairs,
  with the pairs only one arm succeeded, and a 95% paired bootstrap interval
  (seed 3, 10,000 resamples).

Accord's success **is comparable** if the paired difference is at least
**−0.10**: Accord no more than ten points below LemmaScript. The pass mark
doesn't change after the run.

With 40 pairs the interval is wide, and the pass mark uses the point
estimate: the result is exploratory, not a claim of equivalence.

**Reported, outside the pass mark:** checks used; output tokens; failure
kind (no final file, not accepted, or accepted but failing a hidden
example); and, for Accord, whether the final contract determines the result
(no open case).

## Procedure

1. This page merges: E3 is registered.
2. **Pilot**, excluded from the analysis: `t07` and `t10` in both arms with
   Haiku, one repetition, to check timing and the five-check limit. If the
   harness changes, the change and its reason are recorded here before the
   full run.
3. **Full run:** `python run.py --models haiku,sonnet --reps 2`, then
   `python score.py`. Runs, transcripts and `scores.csv` are committed with
   the result. Transcripts have the operator's local paths and user name
   replaced by placeholders.

## Pilot (2026-11-09)

Run after registration, excluded from the analysis, with the harness
unchanged: `t07` and `t10` in both arms with Haiku, one repetition. All four
runs were accepted on their first check and passed every hidden example, in
21 to 25 seconds each, `t10`'s loop invariants in LemmaScript included. One
transcript records a denied command: the check command with its path in
quotes, which the allow rule doesn't match; the agent ran it unquoted next.

**Risk this shows: a ceiling.** If most runs succeed in both arms, the
paired difference is near zero and the pass mark is met because the tasks
don't separate the languages, not because they are equal. A result like that
is reported as comparable on these rules only, and a harder set (longer
rules, more parameters, less exact prose) needs a registration of its own.

The full run has not started.

## Disclosures

- **What ran before registration.** The reference solutions, checked by both
  checkers, and one harness check: `t01-clamp` with Haiku in each arm, both
  accepted on the first check with every hidden example passing. Those two
  runs are not part of E3.
- **Who wrote the material.** Accord's authors wrote the tasks, both reference
  pages and the harness. The pages follow one structure at similar length,
  but an author knows Accord better than LemmaScript.
- **Exposure.** Tesota's rules, with their LemmaScript contracts, and their
  Accord ports are public; a model may have seen either.
- **Only two models, both Anthropic's,** in one agent harness: a result is
  about Claude Haiku 5.5 and Claude Sonnet 5.5 in Claude Code, not agents in
  general.
- **Asymmetries kept on purpose.** Each checker prints its own output:
  Accord's counterexamples and open cases, Dafny's errors on the generated
  `.dfy`. They are what a user of each would see.
- **Asymmetry not removed.** Accord's hidden examples run through the
  interpreter, which honours the agent's `requires`; Node runs TypeScript
  regardless of `//@ requires`. An agent that narrows a `requires` below the
  task's can fail hidden examples in Accord and not in TypeScript.

## Reproduce

Set `ACCORD_E3_ACCORD` to an `accord` binary, `ACCORD_E3_LSC` to the `lsc`
executable, and `ACCORD_Z3` and `ACCORD_CVC5` as for Accord's tests; Node 22.6
or later runs the hidden examples. Then `python score.py --self-test`. CI
doesn't run it, because it needs LemmaScript and Dafny.
