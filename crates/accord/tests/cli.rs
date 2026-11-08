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
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .matches("proved")
            .count(),
        2
    );
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
