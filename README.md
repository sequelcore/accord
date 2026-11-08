# Accord

A programming language whose compiler proves what the code promises.

**Status:** experimental. Nothing is released, and the language doesn't
compile anything yet: the [roadmap](ROADMAP.md) says what's being built
and in what order.

## The idea

When an agent writes code, a person still has to decide whether it does what
they meant. Accord puts what that decision needs into the language itself:
the intent in plain words, the specification, and the evidence. The compiler
is the verifier: it turns each specification into
proof obligations, discharges them with SMT solvers, and reports each one as
proved, refuted with a counterexample, undecided or unsupported. Evidence is
bound to the exact content it checked and goes stale when that changes.
Nothing undecided is reported as proved. The [concepts](docs/concepts.md)
define each term, and the [background](docs/design/background.md) says where
the design comes from.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you state otherwise, any contribution you submit for
inclusion in this work, as defined in the Apache-2.0 license, is dual
licensed as above, without any additional terms or conditions.
