//! A receipt is evidence only if checking the same file again gives the same
//! receipt, byte for byte: the same outcomes, keys, hashes, counterexamples,
//! witnesses, open cases and surviving mutants.

use accord::{Solvers, receipt, verify};

/// Cases whose receipts hold every kind of model the solvers give.
const CASES: [&str; 6] = [
    "proof-outcome",          // witnesses, an open case and surviving mutants
    "decide-fault",           // a counterexample to coverage
    "overflow-fault",         // integer counterexamples
    "list-in-fault",          // a list read back from a model
    "text-accented-fault",    // a text read back from a model
    "decide-otherwise-fault", // a row no input reaches
];

#[test]
fn checking_a_file_twice_gives_the_same_receipt() {
    let solvers = Solvers::from_env()
        .expect("Z3 and cvc5 are available (see conformance/README.md)")
        .with_mutation();
    for case in CASES {
        let path = format!(
            "{}/../../conformance/cases/{case}.accord",
            env!("CARGO_MANIFEST_DIR")
        );
        let source = std::fs::read_to_string(&path).expect("the case");
        let receipt = || {
            let report = verify(&source, &solvers).expect("the case checks");
            receipt::json(&report, case)
        };
        let (first, second) = (receipt(), receipt());
        assert_eq!(first, second, "{case}'s receipt changed between two checks");
    }
}
