//! `accord build`: each case in `conformance/typescript` is a program in its
//! `src` folder. The modules it emits must equal its `out` folder exactly, and
//! a case without an `out` folder must be refused.

use std::fs;
use std::path::{Path, PathBuf};

use accord::{Solvers, build};

/// The files in `dir` with extension `extension`, by name, each with its text.
fn files(dir: &Path, extension: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == extension))
        .map(|path| {
            let name = path.file_name().expect("a name").to_string_lossy();
            (
                name.into_owned(),
                fs::read_to_string(&path).expect("readable file"),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_case_emits_exactly_its_expected_modules() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/typescript");
    let mut cases: Vec<PathBuf> = fs::read_dir(&root)
        .expect("conformance/typescript exists")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no TypeScript cases found");
    for case in cases {
        let sources = files(&case.join("src"), "accord");
        let built = build(&sources, &solvers)
            .unwrap_or_else(|diagnostic| panic!("{}: {diagnostic:?}", case.display()));
        let expected = case.join("out");
        match built.modules {
            None => assert!(
                !expected.exists(),
                "{}: refused, standing {:?}",
                case.display(),
                built.report.standing
            ),
            Some(modules) => {
                assert!(
                    expected.exists(),
                    "{}: emitted, expected a refusal",
                    case.display()
                );
                let got: Vec<(String, String)> = modules
                    .into_iter()
                    .map(|module| (module.name, module.source))
                    .collect();
                let want = files(&expected, "ts");
                let names = |all: &[(String, String)]| -> Vec<String> {
                    all.iter().map(|(name, _)| name.clone()).collect()
                };
                assert_eq!(names(&got), names(&want), "{}", case.display());
                for ((name, got), (_, want)) in got.iter().zip(&want) {
                    assert_eq!(got, want, "{}/{name}", case.display());
                }
            }
        }
    }
}

#[test]
fn structured_data_is_refused_before_anything_is_written() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    for (source, what) in [
        ("record Pair {\n    a: Bool,\n}\n", "records"),
        (
            "enum Id { Numeric(digits: Text) }\n",
            "variants with fields",
        ),
        (
            "enum Light { Red, Green }\npub fn red(l: Light) -> Bool\n{\n    match l {\n        | Red   => true\n        | Green => false\n    }\n}\n",
            "`match`",
        ),
    ] {
        let refused = build(&[("rule.accord".to_string(), source.to_string())], &solvers)
            .expect_err("refused");
        assert!(
            refused
                .message
                .contains(&format!("doesn't write {what} as TypeScript yet")),
            "{refused:?}"
        );
    }
}
