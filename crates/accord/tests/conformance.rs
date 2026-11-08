//! Runs every case in `conformance/cases` and compares each obligation's
//! outcome with the `// expect:` line the case declares.

use std::fs;
use std::path::PathBuf;

use accord::{Outcome, Solvers, verify};

fn cases() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("conformance/cases exists")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "accord"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no conformance cases found");
    paths
}

fn expected(source: &str) -> Vec<String> {
    let line = source.lines().next().unwrap_or_default();
    let list = line
        .strip_prefix("// expect:")
        .expect("a case starts with an `// expect:` line");
    list.split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

fn kind(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Proved => "proved",
        Outcome::Refuted { .. } => "refuted",
        Outcome::Undecided { .. } => "undecided",
        Outcome::Unsupported { .. } => "unsupported",
    }
}

#[test]
fn every_case_gives_its_declared_outcomes() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    let mut failures = Vec::new();
    for path in cases() {
        let source = fs::read_to_string(&path).expect("readable case");
        let want = expected(&source);
        let got: Vec<String> = match verify(&source, &solvers) {
            Ok(results) => results
                .iter()
                .map(|r| kind(&r.outcome).to_string())
                .collect(),
            Err(_) => vec!["error".to_string()],
        };
        if got != want {
            failures.push(format!(
                "{}: expected {want:?}, got {got:?}",
                path.display()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "conformance failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_refutation_names_a_value_for_each_parameter() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    for path in cases() {
        let source = fs::read_to_string(&path).expect("readable case");
        let Ok(results) = verify(&source, &solvers) else {
            continue;
        };
        for result in results {
            if let Outcome::Refuted { counterexample } = &result.outcome {
                assert!(
                    !counterexample.is_empty(),
                    "{}: refutation of `{}` has no counterexample",
                    path.display(),
                    result.clause
                );
            }
        }
    }
}
