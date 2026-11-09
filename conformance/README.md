# Conformance

Each case in [`cases/`](cases) is an Accord file whose first line states the
outcome of each of its obligations, in source order, written before the
compiler code that produces it:

```accord
// expect: proved, refuted
```

Each folder in [`programs/`](programs) is one program of several files. Every
file in it starts with its own `// expect:` line (empty when it declares
nothing to check), and is checked with the other files' declarations in view.

A case may add an `// open:` line as its second line: the openness of each
function with an `ensures`, in source order, as `determined`, `open` or
`undecided`. Without it, openness isn't compared.

`proved`, `refuted`, `undecided` and `unsupported` are obligation outcomes
([concepts](../docs/concepts.md)); `passed` and `failed` are example runs,
listed after every obligation, in source order; `error` means the file must
be rejected before any obligation is generated. A case named `x-fault` or `x-<what>-fault`
plants a fault in the correct case `x`, and must be refuted or fail.

`cargo test --test conformance` runs every case through both solvers. It
needs Z3 and cvc5: on the `PATH` as `z3` and `cvc5`, or named by the
`ACCORD_Z3` and `ACCORD_CVC5` environment variables. Without them the
harness fails rather than skipping.
