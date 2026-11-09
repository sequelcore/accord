//! The `accord` command.

use std::process::ExitCode;

use accord::{Solvers, Standing, diff, receipt, verify};

const USAGE: &str = "usage: accord check <file> [--json]\n       accord diff <old> <new> [--json]    (either file may be `-` for stdin)\n       accord --version";

/// `check`'s exit code follows the file's standing: 0 verified; 1 failed (an
/// obligation refuted or an example failed); 3 incomplete or unverified
/// (nothing failed, but something wasn't proved or nothing was checked).
/// `diff`'s: 0 no contract got weaker; 1 one got weaker or incomparable, or a
/// function was removed; 3 nothing proved weaker, but something couldn't be
/// compared. For both, 2 is a usage error, and 4 means a file was rejected or
/// couldn't be read, or the solvers couldn't be found.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" => {
            println!("accord {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [command, path] if command == "check" => check(path, false),
        [command, path, flag] if command == "check" && flag == "--json" => check(path, true),
        [command, old, new] if command == "diff" => compare(old, new, false),
        [command, old, new, flag] if command == "diff" && flag == "--json" => {
            compare(old, new, true)
        }
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
    // The ledger sits beside the file checked, as a cache nobody commits.
    let ledger = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new(""))
        .join(".accord");
    let solvers = match Solvers::from_env() {
        Ok(solvers) => solvers.with_ledger(&ledger).with_mutation(),
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

/// A file's text, or stdin's for `-`.
fn read(path: &str) -> Result<String, ExitCode> {
    let read = if path == "-" {
        std::io::read_to_string(std::io::stdin())
    } else {
        std::fs::read_to_string(path)
    };
    read.map_err(|error| {
        eprintln!("{path}: {error}");
        ExitCode::from(4)
    })
}

fn compare(old_path: &str, new_path: &str, as_json: bool) -> ExitCode {
    if old_path == "-" && new_path == "-" {
        eprintln!("only one side can be read from stdin\n{USAGE}");
        return ExitCode::from(2);
    }
    let (old, new) = match (read(old_path), read(new_path)) {
        (Ok(old), Ok(new)) => (old, new),
        (Err(code), _) | (_, Err(code)) => return code,
    };
    let solvers = match Solvers::from_env() {
        Ok(solvers) => solvers,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(4);
        }
    };
    let comparison = match diff::diff(&old, &new, &solvers) {
        Ok(comparison) => comparison,
        Err((side, diagnostic)) => {
            let path = match side {
                diff::Side::Old => old_path,
                diff::Side::New => new_path,
            };
            eprintln!("{path}:{}: {}", diagnostic.line, diagnostic.message);
            return ExitCode::from(4);
        }
    };
    if as_json {
        println!("{}", diff::json(&comparison, old_path, new_path));
    } else {
        println!("{}", diff::text(&comparison));
    }
    match comparison.verdict {
        diff::Verdict::Kept => ExitCode::SUCCESS,
        diff::Verdict::Weakened => ExitCode::from(1),
        diff::Verdict::Undecided => ExitCode::from(3),
    }
}
