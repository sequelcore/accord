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
