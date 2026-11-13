//! `accord diff`: planted weakenings are detected, each with what it newly
//! allows, and strengthenings aren't flagged.

use accord::Solvers;
use accord::diff::{Change, Comparison, Loosening, Verdict, diff};

fn compare(old: &str, new: &str) -> Comparison {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    diff(old, new, &solvers).expect("both revisions check")
}

fn only(comparison: &Comparison) -> &Change {
    let [function] = comparison.functions.as_slice() else {
        panic!("{comparison:?}")
    };
    &function.change
}

const ADD: &str = "fn add(a: i32, b: i32) -> i32
    requires a + b < 100 and a + b > 0
    ensures result == a + b
{
    a + b
}
";

#[test]
fn renaming_and_restating_the_same_contract_is_equivalent() {
    let restated = "fn add(x: i32, y: i32) -> i32
    requires x + y > 0
    requires x + y < 100
    ensures result == y + x
{
    x + y
}
";
    let comparison = compare(ADD, restated);
    assert_eq!(only(&comparison), &Change::Equivalent);
    assert_eq!(comparison.verdict, Verdict::Kept);
}

#[test]
fn a_looser_ensures_is_weaker_with_the_result_it_newly_allows() {
    let looser = ADD.replace("ensures result == a + b", "ensures result >= a + b");
    let comparison = compare(ADD, &looser);
    let Change::Weaker(Loosening::Allows(inputs, result)) = only(&comparison) else {
        panic!("{comparison:?}")
    };
    let value = |name: &str| -> i64 {
        inputs
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.parse().ok())
            .expect("an integer input")
    };
    let result: i64 = result.parse().expect("an integer result");
    assert!(result > value("a") + value("b"), "{inputs:?} {result}");
    assert_eq!(comparison.verdict, Verdict::Weakened);
}

#[test]
fn a_deleted_ensures_is_weaker() {
    let deleted = ADD.replace("    ensures result == a + b\n", "");
    assert!(matches!(only(&compare(ADD, &deleted)), Change::Weaker(_)));
}

#[test]
fn a_narrower_requires_is_weaker_with_the_input_it_no_longer_accepts() {
    let narrower = ADD.replace("a + b > 0", "a + b > 10");
    let comparison = compare(ADD, &narrower);
    let Change::Weaker(Loosening::Rejects(inputs)) = only(&comparison) else {
        panic!("{comparison:?}")
    };
    assert_eq!(inputs.len(), 2, "{inputs:?}");
}

#[test]
fn strengthenings_are_not_flagged() {
    // A wider `requires` accepts more inputs and promises the same on them.
    let wider = ADD.replace("a + b > 0", "a + b > -10");
    // From a looser start, a precise `ensures` promises more.
    let looser = ADD.replace("ensures result == a + b", "ensures result >= a + b");
    for (old, new) in [(ADD.to_string(), wider), (looser, ADD.to_string())] {
        let comparison = compare(&old, &new);
        assert_eq!(only(&comparison), &Change::Stronger, "{new}");
        assert_eq!(comparison.verdict, Verdict::Kept);
    }
    // A promise the old contract already implied adds nothing.
    let implied = ADD.replace(
        "    ensures result == a + b\n",
        "    ensures result == a + b\n    ensures result > 0\n",
    );
    assert_eq!(only(&compare(ADD, &implied)), &Change::Equivalent);
}

#[test]
fn two_different_promises_are_incomparable() {
    let old =
        "fn pick(a: Bool, b: Bool) -> Bool\n    ensures result implies a\n{\n    a and b\n}\n";
    let new = old.replace("ensures result implies a", "ensures result implies b");
    let comparison = compare(old, &new);
    assert!(
        matches!(only(&comparison), Change::Incomparable(_)),
        "{comparison:?}"
    );
    assert_eq!(comparison.verdict, Verdict::Weakened);
}

#[test]
fn removed_added_and_renamed_functions() {
    let two = format!("{ADD}\nfn same(a: Bool) -> Bool\n    ensures result == a\n{{\n    a\n}}\n");
    let removed = compare(&two, ADD);
    assert_eq!(removed.functions[1].change, Change::Removed);
    assert_eq!(removed.verdict, Verdict::Weakened);

    let added = compare(ADD, &two);
    assert_eq!(added.functions[1].change, Change::Added);
    assert_eq!(added.verdict, Verdict::Kept);

    let renamed = compare(ADD, &ADD.replace("fn add(", "fn sum("));
    assert_eq!(only(&renamed), &Change::Renamed("add".to_string()));
    assert_eq!(renamed.verdict, Verdict::Kept);
}

#[test]
fn a_changed_signature_or_no_contract_is_reported_as_such() {
    let wider_type = ADD.replace("i32", "i64");
    let changed = compare(ADD, &wider_type);
    assert_eq!(only(&changed), &Change::SignatureChanged);
    assert_eq!(changed.verdict, Verdict::Undecided);

    let bare = "fn same(a: Bool) -> Bool\n{\n    a\n}\n";
    assert_eq!(
        only(&compare(bare, &bare.replace("{\n    a", "{\n    not a"))),
        &Change::NoContract
    );
}

#[test]
fn a_dropped_property_of_the_ported_rule_is_caught() {
    let rule = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/cases/proof-outcome.accord"
    ))
    .expect("the ported rule");
    let dropped = rule.replace(
        "    ensures not cancelled and timed_out implies result == TimedOut\n",
        "",
    );
    let comparison = compare(&rule, &dropped);
    let Change::Weaker(Loosening::Allows(inputs, result)) = only(&comparison) else {
        panic!("{comparison:?}")
    };
    // A timed-out run may now be reported as something else.
    assert!(
        inputs.iter().any(|(n, v)| n == "timed_out" && v == "true"),
        "{inputs:?}"
    );
    assert_ne!(result, "ProofOutcome.TimedOut");
}

#[test]
fn a_machine_on_either_side_is_refused() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    let machine = "machine Switch {
    state { presses: i32 }
    init start() { next { presses: 0 } }
    invariant bounded: presses >= 0
}
";
    let Err((side, refusal)) = diff(ADD, machine, &solvers) else {
        panic!("a machine was compared");
    };
    assert_eq!(side, accord::diff::Side::New);
    assert!(refusal.message.contains("machines aren't compared yet"));
    let Err((side, _)) = diff(machine, ADD, &solvers) else {
        panic!("a machine was compared");
    };
    assert_eq!(side, accord::diff::Side::Old);
}
