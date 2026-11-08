# Conformance

Each case in [`cases/`](cases) is an Accord file whose first line states the
outcome of each of its obligations, in source order, written before the
compiler code that produces it:

```accord
// expect: proved, refuted
```

`proved`, `refuted`, `undecided` and `unsupported` are obligation outcomes
([concepts](../docs/concepts.md)); `error` means the file must be rejected
before any obligation is generated. A case whose name ends in `-fault` plants
a fault in a correct case of the same name, and must be refuted.

`cargo test --test conformance` runs every case through both solvers. It
needs Z3 and cvc5: on the `PATH` as `z3` and `cvc5`, or named by the
`ACCORD_Z3` and `ACCORD_CVC5` environment variables. Without them the
harness fails rather than skipping.
