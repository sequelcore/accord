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
- **Design on merit.** How Tesota or quint-ts solves something is a reference,
  not a template; a Tesota rule is a target to port, not a design to copy.
  Weigh each design choice against comparable tools and where their users
  struggle, with the relevant design skills, and choose the clearest design.
- **No releases yet.** Publishing a package or a release, and announcing
  one, are the maintainers' decisions.

## Checks

The tests need Z3 and cvc5. `scripts/install-solvers.sh <folder>` downloads
the pinned releases, verifies their checksums and prints the `ACCORD_Z3` and
`ACCORD_CVC5` values to export; CI runs the same script. Without the solvers,
the conformance harness fails rather than skipping.

CI runs fmt, clippy and the tests (`.github/workflows/ci.yml`); run them
locally before every push, and push only a finished change: every push runs
CI and holds up review. Linux runs on every push and is the required check.
Windows and macOS run once per pull request, before merging: add the
`full-ci` label (remove and add it again to rerun after a later push).

`main` takes changes only through pull requests, merged as merge commits once
`linux` passes and every conversation is resolved; it can't be force-pushed
or deleted, and nobody bypasses these rules. The toolchain is pinned
in `rust-toolchain.toml`. If `cargo fmt` reports that rustfmt "is not
applicable" to the toolchain, the component is missing on disk: run
`rustup component remove rustfmt --toolchain 1.93.1` and add it again.
