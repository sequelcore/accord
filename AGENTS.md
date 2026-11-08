# Accord

Instructions for agents contributing to Accord, a language and proof-oriented
compiler. The [roadmap](ROADMAP.md) owns what is built and in what order; the
[HQ record](https://github.com/sequelcore/sequel-hq/blob/main/docs/strategy/accord.md)
owns why.

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
- **Internal.** No announcements, publishing or outside contact; each is the
  owner's decision.

## Checks

CI runs the same three commands on Windows, Linux and macOS
(`.github/workflows/ci.yml`); run them before pushing. The toolchain is pinned
in `rust-toolchain.toml`. If `cargo fmt` reports that rustfmt "is not
applicable" to the toolchain, the component is missing on disk: run
`rustup component remove rustfmt --toolchain 1.93.1` and add it again.
