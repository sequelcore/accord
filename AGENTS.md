# Accord

Instructions for agents contributing to Accord, a language and proof-oriented
compiler. The [README](README.md) says why it exists, the [roadmap](ROADMAP.md)
what is built and in what order, and the [concepts](docs/concepts.md) what
each term means.

## Rules

- **Fail closed.** A result claims only what was checked: an obligation is
  proved only when a solver proved it. A timeout, an unknown answer, an
  unsupported construct or a disagreement between solvers is reported as
  such, never as proved.
- **Conformance first.** A construct joins the language with conformance
  cases whose expected outcomes are written before the compiler code, planted
  faults included.
- **Specification semantics are defined, not borrowed.** Where Accord's
  meaning differs from TypeScript's or Rust's (integer width, overflow,
  evaluation order), the reference says so and a case covers it.
- **Grow by the roadmap's target.** Add only what the current stage's target
  needs; the next Tesota rule to port decides the next construct.
- **Independent of Tesota.** No Tesota code or vocabulary in Accord's API.
- **No releases yet.** Publishing a package or a release, and announcing
  one, are the maintainers' decisions.

## Checks

CI runs fmt, clippy and the tests (`.github/workflows/ci.yml`); run them
locally before every push, and push only a finished change: every push runs
CI and holds up review. Linux runs on every push and in the merge queue, and
is the required check. Windows and macOS run once per pull request, before
merging: add the `full-ci` label (remove and add it again to rerun after a
later push). Changes merge through the merge queue. The toolchain is pinned
in `rust-toolchain.toml`. If `cargo fmt` reports that rustfmt "is not
applicable" to the toolchain, the component is missing on disk: run
`rustup component remove rustfmt --toolchain 1.93.1` and add it again.
