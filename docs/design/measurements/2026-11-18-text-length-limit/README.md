# A limit on a text's length: solver answers, 2026-11-18

**Status:** one run, five seeds per solver, of the scripts Accord writes for
twelve small functions, on one machine. It informs how scripts state the
most code points a text holds (#83, slice 11b), and doesn't measure larger
programs.

## Question

`length(text)` is an `i64`, but SMT-LIB's `str.len` is unbounded, so the
solvers can't show that arithmetic on a length stays within `i64`: cvc5
answers with a text 2^63 code points long, and Z3 answers unknown. Every text
a program holds has at most 2^53 − 1 code points, as a JavaScript string
does. Where must a script state that limit for both solvers to prove such
obligations, without a limit the theory contradicts?

## What runs

[`programs/`](programs) holds the functions. `sites.accord` takes a text's
length in each place Accord allows: a parameter (`top`), a record's field
(`named`), an indexed element (`at`), inside `all`, `any` and `count`
(`all_short`, `any_long`, `long_ones`), and through a callee's contract
inside `all` (`widths`). `refute.accord` holds four false contracts.

[`cases/`](cases) holds every script Accord at 55772e0 sent to the solvers
for them, one folder per function, captured with [`capture.sh`](capture.sh)
as Z3. They include the scripts for mutants and for the open-case search.
[`variants.py`](variants.py) writes four variants of each:

| Variant | The script |
| --- | --- |
| none | As Accord wrote it |
| A | A limit on every text constant and every text field of a record constant: the parameters and results |
| B | A limit at every `(str.len X)` an assertion holds, under the guards and `let`s of the quantifiers around it, after expanding each use of a `define-fun` |
| saturate | Every `(str.len X)` replaced by `(ite (<= (str.len X) L) (str.len X) L)` |

[`run.sh`](run.sh) runs each variant with Z3 and cvc5 under seeds 1 to 5,
with Accord's flags (`-t:10000`; `--mbqi --tlimit-per=10000`), and
[`summarize.py`](summarize.py) combines their answers as Accord does: both
`unsat` proves, both `sat` refutes, anything else is undecided. The answers
are in [`results.txt`](results.txt).

## Identity

Accord at 55772e0. Z3 5.3.3 and cvc5 1.4.2 from
`scripts/install-solvers.sh`. Windows 11 Pro, a developer machine whose
background load wasn't controlled.

## Result

| Outcome (none / A / B / saturate) | Scripts |
| --- | --- |
| proved / proved / proved / proved | 19 |
| undecided / undecided / proved / proved | 10 |
| undecided / undecided / undecided / undecided | 7 |
| proved / proved / proved / undecided | 4 |
| undecided / proved / proved / proved | 3 |
| refuted / refuted / refuted / refuted | 2 |

No answer changed between seeds, and neither solver crashed. B costs time in
one script: `long_ones`' open-case search, where Z3 answers unknown in every
variant, took Z3 4.1 to 10.1 s on three of five seeds under B, against about
0.1 s otherwise. The other scripts that ran past 3 s were the three that
only a limit decides, without one: Z3 ran to its 10 s limit.

## What it shows

- **A limit on parameters isn't enough.** It decides the three scripts where
  the length is of a parameter or a field. In the ten that only B proves,
  the length is of a list's element, inside a quantifier or through a
  callee's contract, and cvc5's answer is an element 2^63 code points long.
- **B loses nothing.** Every script that none or A proves, B proves. The only
  `sat` answers B turns into `unsat` are cvc5's for a length beyond the
  limit: the input the limit excludes.
- **A quantified limit doesn't reach cvc5#13161.** In 27 of the B scripts
  the limit sits under a quantifier, always over an index into a list, as
  `(forall ((k Int)) (=> (in range) (let ((w (seq.nth xs k))) (<= (str.len w) L))))`.
  cvc5 with `--mbqi` crashes on a quantifier over a sequence
  ([cvc5/cvc5#13161](https://github.com/cvc5/cvc5/issues/13161)), not over an
  index, and it didn't crash here.
- **Saturating the length loses proofs.** In four mutant scripts of
  `all_short`, whose `requires` bounds each element's length, cvc5 answers
  with texts beyond the limit: their saturated length breaks the mutant's
  contract while the real one wouldn't. None, A and B prove all four.
- **The limit can't be stated of a definition's parameter.** A script calls
  a callee's contract through a `define-fun`. A limit inside one would mean
  every string has at most 2^53 − 1 code points, which the theory of strings
  contradicts, so every claim would be proved. B states it of each use
  instead.
- **Seven scripts stay undecided** whatever the limit: false contracts over a
  `List<Text>`, where cvc5 answers `sat` and Z3 unknown ("incomplete (theory
  seq)"), and `long_ones`' open-case search.

## What it doesn't show

- Larger programs, nested quantifiers (a list of lists of texts), texts in
  an enum's fields, or a quantifier the compiler writes in another shape.
- Linux, where cvc5#13161 grows memory instead of crashing.
- The compiler's own implementation of B, which runs on every script: the
  conformance suite checks that, not this measurement.
- Answer times beyond one run per seed.
