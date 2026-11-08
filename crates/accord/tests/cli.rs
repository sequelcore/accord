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
