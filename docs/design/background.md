# Background

Where Accord's design comes from: the evidence it makes native, how it finds
open cases, and the prior art it studies. The [concepts](../concepts.md)
define the terms used here.

Sources are marked **verified** when their claims were read at the source,
and **reported** when they come from a secondary summary not yet checked.

## Tesota's evidence, made native

[Tesota](https://github.com/sequelcore/tesota) checks the work of coding
agents in existing languages. It rebuilds, with heuristics over TypeScript,
facts that a language could decide itself. Making them native is Accord's
main difference from the roughly 50 languages for agents catalogued on
[agentlanguages.dev](https://agentlanguages.dev) (checked 2026-11-07),
several of which already check contracts with Z3.

| Tesota (`dev` at `76f42821`) | Accord |
| --- | --- |
| Kinds of evidence with their limits, in receipt v3's groups | Each construct has its evidence type: an `ensures` is proved, an `example` is run, a machine is model-checked |
| Model judgment (ClaimCheck, open cases) never counts as evidence | An `intent` clause holds the purpose in prose, linked to the specifications that formalize it; the compiler knows it proves nothing |
| Standing: verified, incomplete, unverified, unchanged, failed; a gap is never a failure | Outcomes per obligation, and standing per definition; the receipt is the compiler's output |
| Contract strength by mutation (`mutation-rule.ts`) | A contract that survives a non-equivalent mutation of its body is reported as weak |
| Weakening flags, heuristic (`weakening-rule.ts`, `test-weakening-rule.ts`) | Weakening decided by the solvers where both revisions are decidable: the old specification implies the new one and not the reverse |
| Staleness by content hashes, which aren't an atomic snapshot | Evidence bound to the hash of exactly what it checked |
| Coverage of changed paths (`proof-cover-rule.ts`) | The compiler knows which definitions carry obligations |
| Design models with planted faults, in separate TLA+ or Quint files | State machines in the same language, with planted faults as a native step |
| Open cases listed by a model; [tesota#796](https://github.com/sequelcore/tesota/issues/796) found that a search doesn't beat asking | Open cases found by the solvers (below) |

## Open cases found by the solvers

[Djinnlang](https://arxiv.org/abs/2609.23954) (Henniger, Chong and Amin,
2026-09-21, verified) requires a specification to be unambiguous: any two
implementations that satisfy it give the same outputs. When a specification
leaves freedom, a solver can produce one input with two permitted results.
That witness is an open case.

Accord doesn't require full determinism. A freedom a person accepts becomes a
permitted freedom; an open case is an ambiguity no permitted freedom covers,
and no definition is verified while one remains. Whether a freedom is
acceptable is the person's decision.

## Prior art

- **[Djinnlang](https://arxiv.org/abs/2609.23954)** (verified):
  specification-only programming with an LLM in the compiler. Two of its
  authors also wrote [claimcheck](https://arxiv.org/abs/2610.17358), the
  method Tesota's ClaimCheck adapts.
- **[Bend 2](https://github.com/bendlang/bend)** (verified): laws written by
  people, proofs written by agents, and its documented trust limits.
- **Vera, Intent, Aver, Thermite and Faber Romanus**, from agentlanguages.dev
  (verified there): contracts checked with Z3, export to Lean or Dafny, and
  several renderings of one representation.
- **JetBrains MPS 2026.1's Projectional Agent Toolkit** (reported):
  structured models exposed to agents over MCP.
- **Unison** (content-addressed definitions) and **Hazel** (typed holes).
- **Clover**, **MutDafny** and **Verified but Vacuous** (reported):
  consistency between descriptions, contracts and code; mutation of
  specifications; verifier acceptance that doesn't bind to the specification.
- **[Kaufman et al.](https://arxiv.org/abs/2607.08885)** (verified): in a
  study of 86 programmers, incorrect LLM-generated assertions were judged
  correctly 49% of the time, against 74% for correct ones, with similar
  confidence. Reading a specification is not enough by itself; what helps a
  person judge it is the question Accord's review view has to answer.
