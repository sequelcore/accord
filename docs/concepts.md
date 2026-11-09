# Concepts

The meaning of each term Accord uses, one concept per term. This page owns
the vocabulary until the compiler's types do: from stage 2, a closed type in
the code owns each set of states below, and this page explains it.

The terms build on Tesota's receipt v3 where the concept is the same, and say
so where they differ.

## Three authorities

Every statement Accord reports comes from exactly one of three authorities,
and they never mix:

| Authority | What it produces | Example |
| --- | --- | --- |
| **Compiler** | Evidence: outcomes of checks it ran | An obligation proved by both solvers |
| **Model** | Opinion: a judgment no check backs | A model says an intent and its contract disagree |
| **Human** | Approval and acceptance: decisions | A person approves an intent, or accepts a change |

Opinion never becomes evidence, and neither becomes approval. The compiler
never records acceptance; that stays with the person.

## What the program says

- **Definition:** a named function, type or state machine. Its identity is
  the hash of its normalized meaning, so a rename doesn't change it.
- **Contract:** a function's `requires` and `ensures`.
- **Invariant:** a property every value of a type, or every reachable state
  of a machine, must have.
- **Specification:** everything that states what must hold: contracts,
  invariants and machine properties. Examples are not part of it.
- **Example:** one concrete call and the result it must give. It states
  intent in concrete terms and is checked by running it.
- **Decision table:** a function body written as ordered rows of a
  condition and a result, where the first row whose condition holds decides.
  It is the body and the specification at once, so a decision isn't written
  twice. The compiler proves that every input reaches a row and reports any
  row that no input can reach.
- **Intent:** a statement, in natural language, of what a definition is for,
  linked to the specifications that formalize it. An intent proves nothing;
  whether its specifications say what it says is a matter of opinion or
  approval, never of evidence. The word is chosen so that readers take it as
  unchecked; "claim" reads as an assertion
  ([probe](design/probes/2026-11-07-intent-and-decision-tables/README.md)).
- **Assumption:** a premise the compiler doesn't prove, such as the behavior
  of an external function. Every assumption is listed in the receipt.
- **Permitted freedom:** a declaration that, for a described set of inputs,
  more than one result is acceptable.

## What the compiler checks

- **Obligation:** one logical statement the compiler derives from the code
  and its specification that must hold: a postcondition on one path, the
  absence of overflow in one addition, a precondition at one call. Some come
  from the specification; others are implicit in the language.
- **Polarity** of an obligation: **universal** when it must hold for every
  admitted input, as a postcondition must, or **existential** when it needs
  some admitted input, as a decision table's row needs one that reaches it.
- **Outcome** of an obligation, one of:
  - **proved:** both solvers proved it. An existential obligation is proved
    by a **witness**, an input for which it holds, reported in source terms;
  - **refuted:** the solvers found a **counterexample**, concrete inputs on
    which it fails, reported in source terms; or, for an existential
    obligation, both proved that no input exists;
  - **undecided:** no proof and no counterexample, with a reason: a timeout,
    an unknown answer, or the solvers disagreeing (which is also raised as
    an alarm);
  - **unsupported:** the obligation uses something outside what the compiler
    can check yet.
- **Example run:** passed or failed.
- **Strength** of a contract: the body is mutated, and each mutant is
  rejected (the contract catches it), surviving (it still proves), equivalent
  (proved to behave the same) or inconclusive. A contract with a surviving
  mutant is **weak**. These are receipt v3's mutation leaves.
- **Weakening:** two revisions of one specification compared: weaker,
  stronger, equivalent, incomparable or undecided. Unlike Tesota's heuristic
  weakening flags, this is decided by the solvers where both revisions are
  decidable.
- **Open case:** an input for which the specification permits more than one
  result and no permitted freedom covers it. It is found by the solvers, with
  its witness: the input and two permitted results.

## Evidence

**Evidence** is the record of one check: its kind, its outcome, and the
hashes of exactly what it checked (the definition, its specification, its
dependencies, the assumptions and the solver versions).

**Freshness** is independent of the outcome: evidence is **current** while
every hash still matches, and **stale** once one changes. Stale evidence
counts for nothing, whatever its outcome was.

## Approval

**Approval** is a person's decision that an intent, a specification, an
assumption or a permitted freedom is the intended one. It is bound to the
hash of what was approved. A change to an approved item makes its approval
stale, so an agent can propose a change to a specification but can't make a
weaker one count without a person.

## Standing

**Standing** summarizes a definition, and a program, from its evidence:

| Standing | Meaning |
| --- | --- |
| **failed** | An obligation is refuted or an example failed |
| **verified** | Every obligation is proved, every example passed, all evidence is current, no open case remains and every assumption is approved |
| **incomplete** | Nothing failed, but something keeps it from verified: an undecided or unsupported obligation, stale evidence, an open case, or an unapproved assumption |
| **unverified** | No current evidence |

An obligation is *proved*; a definition is *verified*. A weak contract is
reported but doesn't change standing, since mutation tries only a sample of
changes. Verified never means the specification captures what the person
meant: that is the intent's question, answered by approval.

## The receipt

The **receipt** is the compiler's report of one check: every piece of
evidence, every opinion it was given, kept apart, the standing, and what was
not checked. It is the same record as text for people and as JSON for agents.
[The receipt](receipt.md) documents what the compiler writes today.
