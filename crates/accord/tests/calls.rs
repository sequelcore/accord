//! A caller's proofs assume its callees' contracts, so it is verified only
//! when every function it calls is, and the receipt names those that aren't.
use accord::{Solvers, Standing, receipt, verify, verify_folder};

const CALLEE: &str = "fn successor(n: i32) -> i32
    requires n < 100
    ensures result == n + 1
{
    n + 2
}
";

const CALLER: &str = "fn twice(n: i32) -> i32
    requires n >= 0 and n < 50
    ensures result == n + 2
{
    successor(successor(n))
}
";

#[test]
fn a_caller_of_an_unverified_function_is_not_verified() {
    let solvers = Solvers::from_env().expect("solvers");
    let report = verify(&format!("{CALLEE}\n{CALLER}"), &solvers).expect("checked");
    let [successor, twice] = &report.definitions[..] else {
        panic!("two definitions");
    };
    assert_eq!(successor.standing, Standing::Failed);
    assert!(
        report
            .verdicts
            .iter()
            .filter(|v| v.function == "twice")
            .all(|v| matches!(v.outcome, accord::Outcome::Proved { .. })),
        "its own obligations hold through the callee's contract"
    );
    assert_eq!(twice.standing, Standing::Incomplete);
    assert_eq!(twice.unverified_callees, ["successor"]);
    let json: serde_json::Value =
        serde_json::from_str(&receipt::json(&report, "calls.accord")).expect("JSON");
    let findings = json["diagnostics"].as_array().expect("diagnostics");
    assert!(
        findings
            .iter()
            .any(|d| d["code"] == "ACC0014" && d["function"] == "twice"),
        "{findings:?}"
    );
    assert!(receipt::text(&report).contains("twice: incomplete (calls `successor`, not verified)"));
}

#[test]
fn a_file_is_judged_with_the_callees_another_file_declares() {
    let solvers = Solvers::from_env().expect("solvers");
    let files = [
        ("callee.accord".to_string(), CALLEE.to_string()),
        ("caller.accord".to_string(), CALLER.to_string()),
    ];
    let report = verify_folder(&files, Some("caller.accord"), &solvers).expect("checked");
    let names: Vec<&str> = report.definitions.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        ["twice"],
        "only the checked file's functions are reported"
    );
    assert!(report.verdicts.iter().all(|v| v.function == "twice"));
    assert_eq!(report.definitions[0].standing, Standing::Incomplete);
    assert_eq!(report.definitions[0].unverified_callees, ["successor"]);
}

#[test]
fn a_witness_through_a_call_names_only_parameters() {
    let solvers = Solvers::from_env().expect("solvers");
    let source = "fn successor(n: i32) -> i32
    requires n < 100
    ensures result == n + 1
{
    n + 1
}

fn above_fifty(n: i32) -> Bool
    requires n < 100
decide {
    | successor(n) > 50 => true
    | otherwise         => false
}
";
    let report = verify(source, &solvers).expect("checked");
    let witnesses: Vec<_> = report
        .verdicts
        .iter()
        .filter_map(|v| match &v.outcome {
            accord::Outcome::Proved {
                witness: Some(inputs),
            } => Some(inputs),
            _ => None,
        })
        .collect();
    assert_eq!(witnesses.len(), 2, "both rows are reached");
    for inputs in witnesses {
        let names: Vec<&str> = inputs.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["n"]);
    }
}
