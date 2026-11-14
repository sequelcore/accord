//! Machine actions follow codes and evidence references, never prose matching.
use accord::{Solvers, receipt, verify};
use std::process::Command;

#[test]
fn a_refutation_has_a_stable_code_action_and_resolvable_evidence_reference() {
    let solvers = Solvers::from_env().expect("solvers");
    let report = verify(
        "fn wrong(a: Bool) -> Bool\n ensures result == a\n{ not a }",
        &solvers,
    )
    .unwrap();
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
    let diagnostic = &j["diagnostics"][0];
    assert_eq!(diagnostic["code"], "ACC0002");
    assert_eq!(diagnostic["action"], "inspect_counterexample");
    assert_eq!(diagnostic["authority"], "compiler");
    assert_eq!(diagnostic["line"], 2);
    let evidence = j
        .pointer(diagnostic["evidence"].as_str().unwrap())
        .expect("evidence exists");
    assert_eq!(evidence["outcome"], "refuted");
    assert_eq!(evidence["counterexample"][0]["type"]["kind"], "bool");
}

#[test]
fn json_rejection_and_missing_prerequisites_are_structured_without_a_proof() {
    let root = format!(
        "{}/../../conformance/cases/unknown-name.accord",
        env!("CARGO_MANIFEST_DIR")
    );
    let cases = [
        (root.as_str(), false, "ACC0001", "rejected"),
        (
            "missing-input-for-diagnostics.accord",
            false,
            "ACC0011",
            "unavailable",
        ),
        (root.as_str(), true, "ACC0009", "unavailable"),
    ];
    for (path, missing_solver, code, status) in cases {
        let mut command = Command::new(env!("CARGO_BIN_EXE_accord"));
        command.args(["check", path, "--json"]);
        if missing_solver {
            command.env("ACCORD_Z3", "no-such-solver-for-diagnostics");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4));
        let j: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("structured error on stdout");
        assert_eq!(j["receipt"], 5);
        assert_eq!(j["status"], status);
        assert!(j["standing"].is_null());
        assert!(j["definitions"].as_array().unwrap().is_empty());
        assert_eq!(j["diagnostics"][0]["code"], code);
        assert!(j["diagnostics"][0]["evidence"].is_null());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn gaps_freedoms_examples_and_survivors_have_consistent_machine_routes() {
    let solvers = Solvers::from_env().unwrap().with_mutation();
    let cases = [
        (
            "fn f(n: i64) -> Bool\n ensures result implies n >= 0\n{ n > 0 }",
            vec!["ACC0006", "ACC0007"],
        ),
        ("fn f(a: Bool) -> Bool { a }", vec!["ACC0012"]),
        (
            "fn f() -> Bool { true }\nexample f() == false",
            vec!["ACC0005"],
        ),
        (
            "fn f(n: Bool) -> Bool\n ensures result implies n\n open when n\n{ false }",
            vec![],
        ),
    ];
    for (source, expected) in cases {
        let report = verify(source, &solvers).unwrap();
        let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
        let codes: Vec<_> = j["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["code"].as_str().unwrap())
            .collect();
        for code in expected {
            assert!(codes.contains(&code), "{j}");
        }
        for finding in j["diagnostics"].as_array().unwrap() {
            assert!(j.pointer(finding["evidence"].as_str().unwrap()).is_some());
            assert!(receipt::text(&report).contains(finding["code"].as_str().unwrap()));
        }
        if source.contains("open when") {
            assert!(!codes.contains(&"ACC0006"));
            assert_eq!(j["definitions"][0]["openness"]["free"][0], "open when n");
            assert!(receipt::text(&report).contains("open when n"));
        }
    }
}

#[test]
fn catalog_is_unique_and_projection_handles_unsettled_and_unsupported_outcomes() {
    use accord::{DiagnosticCode, Openness, Outcome, Standing};
    let names: std::collections::BTreeSet<_> =
        DiagnosticCode::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names.len(), DiagnosticCode::ALL.len());
    let solvers = Solvers::from_env().unwrap().with_mutation();
    let base = verify(
        "fn f(a: Bool) -> Bool\n ensures result == a\n{ a }",
        &solvers,
    )
    .unwrap();
    // Adapter coverage: unsupported obligations have no current source producer.
    for (outcome, code) in [
        (
            Outcome::Undecided {
                reason: "arbitrary wording".into(),
            },
            "ACC0003",
        ),
        (
            Outcome::Unsupported {
                reason: "arbitrary wording".into(),
            },
            "ACC0004",
        ),
    ] {
        let mut report = base.clone();
        report.verdicts[0].outcome = outcome;
        report.definitions[0].standing = Standing::Incomplete;
        report.standing = Standing::Incomplete;
        report.openness[0].openness = Openness::Undecided {
            reason: "another explanation".into(),
        };
        report.strength[0].undecided = 1;
        report.strength[0].not_tried = 2;
        let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
        let codes: Vec<_> = j["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["code"].as_str().unwrap())
            .collect();
        assert!(codes.contains(&code));
        assert!(codes.contains(&"ACC0010"));
        assert!(codes.contains(&"ACC0008"));
        assert_eq!(j["standing"], "incomplete");
        let limits = &j["definitions"][0]["not_proved"];
        assert!(
            limits
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["kind"] == "mutation_incomplete")
        );
    }
}

#[test]
fn an_empty_program_has_a_no_evidence_diagnostic() {
    let solvers = Solvers::from_env().unwrap();
    let report = verify("enum Flag { On, Off }", &solvers).unwrap();
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "empty")).unwrap();
    assert_eq!(j["standing"], "unverified");
    assert_eq!(j["diagnostics"][0]["code"], "ACC0012");
    assert_eq!(
        j.pointer(j["diagnostics"][0]["evidence"].as_str().unwrap())
            .unwrap(),
        &serde_json::json!([])
    );
}
