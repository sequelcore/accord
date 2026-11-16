//! Contract strength: planted weak contracts let mutants survive, each with
//! an input on which the code and the mutant differ; strong ones don't.

use accord::{Report, Solvers, Strength, verify};

fn strength(source: &str) -> Vec<Strength> {
    let solvers = Solvers::from_env()
        .expect("Z3 and cvc5 are available (see conformance/README.md)")
        .with_mutation();
    let report: Report = verify(source, &solvers).expect("the file checks");
    report.strength
}

#[test]
fn a_weak_contract_lets_a_differing_mutant_survive() {
    // `implies` only promises one direction: `n >= 0` would also do.
    let weak = strength(
        "fn positive(n: i64) -> Bool
    ensures result implies n >= 0
{
    n > 0
}
",
    );
    let [positive] = weak.as_slice() else {
        panic!("{weak:?}")
    };
    assert!(!positive.survivors.is_empty(), "{positive:?}");
    for survivor in &positive.survivors {
        assert_ne!(survivor.original, survivor.mutant, "{survivor:?}");
        assert_eq!(survivor.inputs.len(), 1, "{survivor:?}");
    }
}

#[test]
fn a_contract_that_ignores_membership_lets_a_negated_in_survive() {
    // The contract pins `read_only` and says nothing about which tools are
    // readers.
    let weak = strength(
        "fn may_write(tool: Text, read_only: Bool) -> Bool
    ensures read_only implies not result
{
    not read_only and not tool in [\"read\", \"ls\"]
}
",
    );
    let [may_write] = weak.as_slice() else {
        panic!("{weak:?}")
    };
    let negated = may_write
        .survivors
        .iter()
        .find(|survivor| survivor.change == "line 4: `in` became `not in`")
        .unwrap_or_else(|| panic!("{may_write:?}"));
    assert_ne!(negated.original, negated.mutant, "{negated:?}");
}

#[test]
fn a_contract_that_pins_membership_catches_a_negated_in() {
    let strong = strength(
        "fn reader(tool: Text) -> Bool
    ensures result implies tool in [\"read\", \"ls\"]
    ensures tool in [\"read\", \"ls\"] implies result
{
    tool in [\"read\", \"ls\"]
}
",
    );
    let [reader] = strong.as_slice() else {
        panic!("{strong:?}")
    };
    assert!(reader.survivors.is_empty(), "{reader:?}");
    assert_eq!(reader.caught, reader.tried, "{reader:?}");
    assert!(reader.tried > 0, "{reader:?}");
}

#[test]
fn a_strong_contract_catches_or_proves_equivalent_every_mutant() {
    let strong = strength(
        "fn larger(a: i64, b: i64) -> i64
    ensures result >= a and result >= b
    ensures result == a or result == b
{
    if a >= b { a } else { b }
}
",
    );
    let [larger] = strong.as_slice() else {
        panic!("{strong:?}")
    };
    assert!(larger.survivors.is_empty(), "{larger:?}");
    assert!(larger.caught > 0, "{larger:?}");
    // `a >= b` to `a > b` returns the same value when they're equal.
    assert!(larger.equivalent > 0, "{larger:?}");
    assert_eq!(larger.undecided, 0, "{larger:?}");
    assert_eq!(
        larger.tried,
        larger.caught + larger.equivalent,
        "{larger:?}"
    );
}

#[test]
fn a_function_without_a_contract_is_not_mutated() {
    assert!(strength("fn same(a: Bool) -> Bool\n{\n    a\n}\n").is_empty());
}

#[test]
fn tesotas_proof_outcome_contract_allows_cancelled_for_a_failed_run() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/cases/proof-outcome.accord"
    ))
    .expect("the ported rule");
    let found = strength(&source);
    let [rule] = found.as_slice() else {
        panic!("{found:?}")
    };
    println!("{rule:#?}");
    // Its six properties say when the result is `Failed`, but not that a
    // failed run can't be reported `Cancelled`.
    let cancelled = rule
        .survivors
        .iter()
        .find(|s| s.change.contains("gives `Cancelled` instead of `Failed`"))
        .unwrap_or_else(|| panic!("{rule:#?}"));
    assert_eq!(cancelled.original, "ProofOutcome.Failed");
    assert_eq!(cancelled.mutant, "ProofOutcome.Cancelled");
    assert!(
        cancelled
            .inputs
            .iter()
            .any(|(n, v)| n == "started" && v == "true"),
        "{cancelled:?}"
    );
}

#[test]
fn a_mutant_that_breaks_the_contract_through_its_own_recursion_is_caught() {
    // The mutant `+ 2` returns 2n, which breaks the contract at n = 2. The
    // solvers find that with the call to itself judged by the contract, and
    // the replay must judge it the same way. So must `steps(n - 0)`: from
    // `steps(n) <= n + 1` the contract gets `n + 2`.
    let loose = strength(
        "fn steps(n: i32) -> i32
    requires n >= 0 and n <= 1000
    decreases n
    ensures result <= n + 1
{
    if n == 0 { 0 } else { steps(n - 1) + 1 }
}
",
    );
    let [steps] = loose.as_slice() else {
        panic!("{loose:?}")
    };
    assert_eq!(steps.undecided, 0, "{steps:?}");
    assert_eq!(steps.caught, 2, "{steps:?}");
}

#[test]
fn a_mutant_that_breaks_another_obligation_is_rejected_not_surviving() {
    // Under this loose contract, four mutants break no `ensures` the solvers
    // can show, yet none is a program the compiler would accept: each calls
    // `steps` below zero. Only the two literal changes in the result are
    // real survivors.
    let loose = strength(
        "fn steps(n: i32) -> i32
    requires n >= 0 and n <= 1000
    decreases n
    ensures result <= n + 1
{
    if n == 0 { 0 } else { steps(n - 1) + 1 }
}
",
    );
    let [steps] = loose.as_slice() else {
        panic!("{loose:?}")
    };
    assert_eq!(steps.rejected, 4, "{steps:?}");
    let changes: Vec<&str> = steps.survivors.iter().map(|s| s.change.as_str()).collect();
    assert_eq!(
        changes,
        ["line 6: `0` became `1`", "line 6: `1` became `0`"],
        "{steps:?}"
    );
    assert_eq!(
        steps.tried,
        steps.caught + steps.rejected + steps.survivors.len()
    );
}

#[test]
fn a_mutant_that_never_ends_is_rejected_by_termination() {
    // The contract can't tell `drain(n - 0)` from the code, since every call
    // gives 0, yet that mutant never ends: its termination obligation
    // rejects it, so it is never reported as surviving with a result it
    // never gives. The four that call `drain` below zero are rejected too,
    // and the contract catches the one that returns 1.
    let tail = strength(
        "fn drain(n: i32) -> i32
    requires n >= 0
    decreases n
    ensures result == 0
{
    if n == 0 { 0 } else { drain(n - 1) }
}
",
    );
    let [drain] = tail.as_slice() else {
        panic!("{tail:?}")
    };
    assert_eq!(drain.undecided, 0, "{drain:?}");
    assert_eq!(drain.caught, 1, "{drain:?}");
    assert_eq!(drain.rejected, 5, "{drain:?}");
    assert!(drain.survivors.is_empty(), "{drain:?}");
}
