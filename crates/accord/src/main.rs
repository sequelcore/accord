//! The `accord` command.

use std::process::ExitCode;

use accord::{Solvers, Standing, receipt, verify};

const USAGE: &str = "usage: accord check <file> [--json]\n       accord --version";

/// Exit codes follow the file's standing: 0 verified; 1 failed (an
/// obligation refuted or an example failed); 3 incomplete or unverified
/// (nothing failed, but something wasn't proved or nothing was checked). 2 is
/// a usage error; 4 means the file was rejected or couldn't be read, or the
/// solvers couldn't be found.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" => {
            println!("accord {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [command, path] if command == "check" => check(path, false),
        [command, path, flag] if command == "check" && flag == "--json" => check(path, true),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check(path: &str, as_json: bool) -> ExitCode {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::from(4);
        }
    };
    let solvers = match Solvers::from_env() {
        Ok(solvers) => solvers,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(4);
        }
    };
    let report = match verify(&source, &solvers) {
        Ok(report) => report,
        Err(diagnostic) => {
            eprintln!("{path}:{}: {}", diagnostic.line, diagnostic.message);
            return ExitCode::from(4);
        }
    };
    if as_json {
        println!("{}", receipt::json(&report, path));
    } else {
        println!("{}", receipt::text(&report));
    }
    match report.standing {
        Standing::Verified => ExitCode::SUCCESS,
        Standing::Failed => ExitCode::from(1),
        Standing::Incomplete | Standing::Unverified => ExitCode::from(3),
    }
}
