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

A machine's obligations come in source order with its functions':

- for each `init`, in order: that some input is allowed, when it has a
  `requires`; that its state meets every invariant and helper invariant;
  then its arithmetic and decision tables, as a function's;
- for each event, in order: that it preserves every invariant and helper
  invariant; that it is allowed in some state an `init` reaches; then its
  arithmetic and decision tables, as a function's, each assuming the
  invariants and helper invariants;
- its observers, as functions.

An invariant is refuted only by a trace from an `init`, which the explorer
finds and the interpreter replays. Where the solvers refute an obligation
from a state no trace reaches, its outcome is `not_inductive`. An event is
allowed in a reachable state when the explorer reaches one where its
`requires` holds. A machine's `example` is listed as `passed` or `failed`
with the other examples.

`cargo test --test conformance` runs every case through both solvers. It
needs Z3 and cvc5: on the `PATH` as `z3` and `cvc5`, or named by the
`ACCORD_Z3` and `ACCORD_CVC5` environment variables. Without them the
harness fails rather than skipping.

## TypeScript output

Each folder in [`typescript/`](typescript) is one program in `src/`, and the
modules `accord build` must write for it in `out/`, byte for byte, written
before the code generator. A folder without `out/` must be refused.
`cargo test --test typescript` compares them. The differential test in
`crates/accord/src/typescript/differential.rs` then runs every program here
that builds in Node and compares each call with the interpreter
([TypeScript reference](../docs/typescript.md)).

The ports in [`programs/tesota/`](programs/tesota) carry Tesota's file names,
so their modules replace Tesota's own; `model-rule.accord` holds only the
type `verdict-rule` reads, so its module replaces nothing.
`scripts/check-tesota.sh <empty folder>` clones Tesota at the commit they
come from and runs its typecheck
and tests twice: with its rules as written, then with the generated modules
in their place. It needs `bun` and network access, so CI doesn't run it.

## axios-retry request predicates

[`programs/axios-retry/`](programs/axios-retry) ports `isRetryableError`,
`isSafeRequestError` and `isIdempotentRequestError` from axios-retry 4.5.0 at
commit `a748627fdc5e22e659055b6cfb44e35ad5801bf4`; the project is archived.
An `AxiosError` arrives as its fields. Each request
predicate calls the retry rule, and its contract restates it, since a
specification can't call a function yet. `axios-retry-status-fault` widens the server-error range to 600 under
the unchanged contract.

[`adapter.patch`](programs/axios-retry/adapter.patch) is written by hand: it
makes axios-retry's own `src/index.ts` read those fields and call the
generated `retry.ts`, and keeps the generated file out of its linter.
`scripts/check-axios-retry.sh <empty folder>` runs axios-retry's lint and
specs as released and with the generated predicates in place. Two of its
specs bound elapsed time from above and fail now and then on a slow machine
in both runs; the check accepts those two only. It needs network access and
Node 20, so CI doesn't run it. The port is licensed under Apache-2.0;
attribution is in [`programs/axios-retry/NOTICE`](programs/axios-retry/NOTICE)
and the source header.

## opossum circuit breaker

[`programs/opossum/`](programs/opossum) ports the state and decisions of
opossum's `CircuitBreaker` from opossum 10.0.0 at commit
`decbedf63d7815049233e544dcb351590ff0c84e` as a machine, with two faults,
`opossum-admits-call-fault` and `opossum-trip-fault`, in [`cases/`](cases). Emitted events, timers, the cache,
the semaphore, the rolling statistics and fallbacks stay in opossum. Where
opossum's behaviour isn't what its states suggest, the port keeps it and
claims no more: any success while half-open closes the breaker, a late one
included. Two divergences are deliberate: the error rate is compared exactly,
so `errorThresholdPercentage` must be a whole number, and a restored state
that says it is both closed and half-open is rejected.

[`adapter.patch`](programs/opossum/adapter.patch) is written by hand: it makes
opossum's own `lib/circuit.js` keep its state in the generated `breaker.ts`,
copied into opossum as `lib/breaker.mts` (in a `"type": "commonjs"` package,
Node loads it only under that extension), and ask it for each decision. It
throws a `TypeError` at construction for a fractional threshold.
`scripts/check-opossum.sh <empty folder>` runs opossum's lint and tests as
released and with the generated module in place. It needs git, Node 22 or
later and network access, so CI doesn't run it. On the machine it was written
on, test 331, a rolling window of 10 ms, and tests 547 to 549, health-check
timers that fire after their test ended, fail in both runs; the check accepts
those only. The port is licensed under Apache-2.0; attribution is in
[`programs/opossum/NOTICE`](programs/opossum/NOTICE).

## Cedar partial authorization

[`programs/cedar/`](programs/cedar) ports the Boolean decision in Cedar's
`PartialResponse::decision` at commit
`5af7b8cd78e4e0da3ae23f1715023fff8e62941d`. Its 16 examples cover every
combination of satisfied and residual permits and forbids. `Unknown` is a
domain result, not an undecided proof. The three `cedar-partial-decision-*-fault`
cases plant incorrect decisions under the unchanged contract.

The port checks only the decision from supplied collection-nonemptiness flags,
not policy evaluation or construction of those collections. It is licensed
under Apache-2.0; attribution is in [`programs/cedar/NOTICE`](programs/cedar/NOTICE)
and the source headers. The rest of Accord retains its own license.
