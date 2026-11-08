//! The `accord` command.

use std::process::ExitCode;

use accord::{Outcome, Solvers, verify};

const USAGE: &str = "usage: accord check <file>\n       accord --version";

/// Exit codes: 0 every obligation proved; 1 one was refuted; 2 usage error;
/// 3 none refuted but one undecided or unsupported; 4 the file was rejected
/// or couldn't be read, or the solvers couldn't be found.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" => {
            println!("accord {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [command, path] if command == "check" => check(path),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check(path: &str) -> ExitCode {
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
    let verdicts = match verify(&source, &solvers) {
        Ok(verdicts) => verdicts,
        Err(diagnostic) => {
            eprintln!("{path}:{}: {}", diagnostic.line, diagnostic.message);
            return ExitCode::from(4);
        }
    };
    let (mut refuted, mut open) = (false, false);
    for verdict in &verdicts {
        let place = format!(
            "{} (line {}): {}",
            verdict.function, verdict.line, verdict.clause
        );
        match &verdict.outcome {
            Outcome::Proved => println!("proved       {place}"),
            Outcome::Refuted { counterexample } => {
                refuted = true;
                let values: Vec<String> = counterexample
                    .iter()
                    .map(|(n, v)| format!("{n} = {v}"))
                    .collect();
                println!(
                    "refuted      {place}\n             fails with {}",
                    values.join(", ")
                );
            }
            Outcome::Undecided { reason } => {
                open = true;
                println!("undecided    {place}\n             {reason}");
            }
            Outcome::Unsupported { reason } => {
                open = true;
                println!("unsupported  {place}\n             {reason}");
            }
        }
    }
    match (refuted, open) {
        (true, _) => ExitCode::from(1),
        (false, true) => ExitCode::from(3),
        (false, false) => ExitCode::SUCCESS,
    }
}
