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
    assert_eq!(receipt["receipt"], 1);
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
