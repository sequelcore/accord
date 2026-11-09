//! Black-box checks of the `accord` command.

use std::process::Command;

fn accord(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_accord"))
        .args(args)
        .output()
        .expect("the accord binary runs")
}

#[test]
fn version_prints_the_package_version() {
    let output = accord(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("accord {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_arguments_are_a_usage_error() {
    let output = accord(&["--nope"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}

#[test]
fn no_arguments_is_a_usage_error() {
    assert_eq!(accord(&[]).status.code(), Some(2));
}

fn case(name: &str) -> String {
    format!(
        "{}/../../conformance/cases/{name}.accord",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn check_exits_0_when_every_obligation_is_proved() {
    let output = accord(&["check", &case("choice")]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.lines().filter(|l| l.starts_with("proved")).count(),
        2
    );
    assert!(
        stdout.contains("standing     the file: verified"),
        "{stdout}"
    );
}

#[test]
fn check_json_writes_the_versioned_receipt() {
    let output = accord(&["check", &case("example-fault"), "--json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the receipt is JSON");
    assert_eq!(receipt["receipt"], 2);
    assert_eq!(receipt["standing"], "failed");
    let larger = &receipt["definitions"][0];
    assert_eq!(larger["name"], "larger");
    assert_eq!(larger["standing"], "failed");
    assert_eq!(larger["hash"].as_str().map(str::len), Some(64));
    let obligations = larger["obligations"].as_array().expect("obligations");
    assert_eq!(obligations.len(), 2);
    assert!(obligations.iter().all(|o| o["outcome"] == "proved"));
    assert_eq!(larger["examples"][0]["run"], "failed");
    assert_eq!(larger["examples"][0]["reason"], "it gives 5, not 2");
    assert_eq!(larger["examples"][1]["run"], "passed");
    // Both `ensures` together pin the result, so no mutant survives.
    assert!(larger["strength"]["tried"].as_u64() > Some(0), "{larger}");
    assert_eq!(
        larger["strength"]["survivors"].as_array().map(Vec::len),
        Some(0)
    );
}

#[test]
fn check_reports_intent_as_unchecked_and_does_not_count_it_as_evidence() {
    let output = accord(&["check", &case("intent"), "--json"]);
    assert_eq!(output.status.code(), Some(3));
    let receipt: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the receipt is JSON");
    let definitions = receipt["definitions"].as_array().expect("definitions");
    assert_eq!(definitions[0]["intent"], "Returns false.");
    assert_eq!(definitions[0]["standing"], "verified");
    assert_eq!(
        definitions[1]["intent"],
        "This statement has not been checked."
    );
    assert_eq!(definitions[1]["standing"], "unverified");
    assert!(definitions[1]["obligations"].as_array().unwrap().is_empty());
    assert!(definitions[2].get("intent").is_some_and(|v| v.is_null()));

    let text = accord(&["check", &case("intent")]);
    assert_eq!(text.status.code(), Some(3));
    let text = String::from_utf8(text.stdout).expect("text receipt");
    assert!(
        text.contains("intent       verified_flag (unchecked): Returns false."),
        "{text}"
    );
    assert!(
        text.contains("intent       intent_only (unchecked): This statement has not been checked."),
        "{text}"
    );
    assert!(
        text.contains("intent       without_intent: not declared"),
        "{text}"
    );
}

#[test]
fn check_exits_3_when_nothing_is_checked() {
    let dir = std::env::temp_dir().join(format!("accord-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    let file = dir.join("nothing.accord");
    std::fs::write(&file, "fn same(a: Bool) -> Bool\n{\n    a\n}\n").expect("written");
    let output = accord(&["check", &file.to_string_lossy()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stdout).contains("unverified"));
}

#[test]
fn check_exits_1_and_shows_the_counterexample_when_one_is_refuted() {
    let output = accord(&["check", &case("identity-fault")]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("refuted"), "{stdout}");
    assert!(stdout.contains("fails with a = "), "{stdout}");
}

#[test]
fn check_shows_a_witness_for_each_reached_row_and_says_when_no_input_reaches_one() {
    let output = accord(&["check", &case("decide-otherwise-fault")]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reached with light = Light."), "{stdout}");
    assert!(stdout.contains("no input reaches it"), "{stdout}");
}

#[test]
fn check_exits_1_and_says_why_when_an_example_fails() {
    let output = accord(&["check", &case("example-fault")]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failed"), "{stdout}");
    assert!(stdout.contains("it gives 5, not 2"), "{stdout}");
}

#[test]
fn check_exits_4_and_names_the_line_when_the_file_is_rejected() {
    let output = accord(&["check", &case("unknown-name")]);
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains(":3: unknown name `b`"));
}

#[test]
fn check_exits_4_when_the_file_is_missing() {
    assert_eq!(
        accord(&["check", "no-such-file.accord"]).status.code(),
        Some(4)
    );
}

fn write(dir: &std::path::Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("written");
    path.to_string_lossy().into_owned()
}

#[test]
fn diff_exits_by_whether_a_contract_got_weaker() {
    let dir = std::env::temp_dir().join(format!("accord-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    let strict = write(
        &dir,
        "strict.accord",
        "fn same(a: Bool) -> Bool\n    ensures result == a\n{\n    a\n}\n",
    );
    let loose = write(
        &dir,
        "loose.accord",
        "fn same(a: Bool) -> Bool\n    ensures result or not result\n{\n    a\n}\n",
    );
    let broken = write(
        &dir,
        "broken.accord",
        "fn same(a: Bool) -> Bool\n{\n    b\n}\n",
    );
    let weaker = accord(&["diff", &strict, &loose]);
    let stronger = accord(&["diff", &loose, &strict]);
    let json = accord(&["diff", &strict, &loose, "--json"]);
    let both_stdin = accord(&["diff", "-", "-"]);
    let missing = accord(&["diff", &strict, "no-such-file.accord"]);
    let rejected = accord(&["diff", &strict, &broken]);
    let folder = accord(&["diff", &strict, &dir.to_string_lossy()]);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(weaker.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&weaker.stdout).contains("weaker       same"));
    assert_eq!(stronger.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&stronger.stdout).contains("stronger     same"));

    assert_eq!(json.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&json.stdout).expect("JSON");
    assert_eq!(document["comparison"], 1);
    assert_eq!(document["verdict"], "weakened");
    assert_eq!(document["functions"][0]["change"], "weaker");
    assert!(document["functions"][0]["loosening"]["allows"].is_object());

    assert_eq!(both_stdin.status.code(), Some(2));
    assert_eq!(missing.status.code(), Some(4));
    assert_eq!(rejected.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("broken.accord:3: unknown name `b`")
    );
    assert_eq!(folder.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&folder.stderr)
            .contains("is a folder; `accord diff` compares two files")
    );
}

#[test]
fn check_takes_a_folder_as_one_program() {
    let folder = format!(
        "{}/../../conformance/programs/shared-enum",
        env!("CARGO_MANIFEST_DIR")
    );
    let whole = accord(&["check", &folder]);
    let stdout = String::from_utf8_lossy(&whole.stdout);
    assert_eq!(whole.status.code(), Some(3), "{stdout}");
    assert!(
        stdout.contains("standing     must_stop (stop.accord): verified"),
        "{stdout}"
    );
    assert!(
        stdout.contains("standing     the folder: incomplete"),
        "{stdout}"
    );

    // A file on its own doesn't see its folder: `Light` is declared beside it.
    let alone = accord(&["check", &format!("{folder}/next.accord")]);
    assert_eq!(alone.status.code(), Some(4));
    let stderr = String::from_utf8_lossy(&alone.stderr);
    assert!(
        stderr.contains("next.accord:3: unknown type `Light`"),
        "{stderr}"
    );

    let twice = accord(&[
        "check",
        &format!(
            "{}/../../conformance/programs/duplicate-name",
            env!("CARGO_MANIFEST_DIR")
        ),
    ]);
    assert_eq!(twice.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&twice.stderr)
            .contains("b.accord:2: `same` is declared more than once")
    );
}
