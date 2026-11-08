# Accord

A programming language and a proof-oriented compiler for humans and coding
agents working together.

**Status:** experimental. Nothing is released, and the language doesn't
compile anything yet: the [roadmap](ROADMAP.md) says what's being built
and in what order.

## The idea

Accord bets that the human, the agent and the language work best as one
system, when both sides understand the same intent, specifications and
evidence. The compiler is the verifier: it turns each specification into
proof obligations, discharges them with SMT solvers, and reports each one as
proved, refuted with a counterexample, undecided or unsupported. Evidence is
bound to the exact content it checked and goes stale when that changes.
Nothing undecided is reported as proved. The [concepts](docs/concepts.md)
define each term.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you state otherwise, any contribution you submit for
inclusion in this work, as defined in the Apache-2.0 license, is dual
licensed as above, without any additional terms or conditions.
