//! The `accord` command.

use std::process::ExitCode;

use std::path::Path;

use accord::{
    Diagnostic, DiagnosticCode, Solvers, Standing, build, diff, receipt, verify, verify_folder,
};

const USAGE: &str = "usage: accord check <file or folder> [--json]\n       accord diff <old> <new> [--json]    (either file may be `-` for stdin)\n       accord build <file or folder> --out <folder>\n       accord --version";

/// `check`'s exit code follows the file's standing: 0 verified; 1 failed (an
/// obligation refuted or an example failed); 3 incomplete or unverified
/// (nothing failed, but something wasn't proved or nothing was checked).
/// `build`'s: 0 every module written; 1 failed and 3 not every obligation
/// proved, as for `check`, with nothing written.
/// `diff`'s: 0 no contract got weaker; 1 one got weaker or incomparable, or a
/// function was removed; 3 nothing proved weaker, but something couldn't be
/// compared. For both, 2 is a usage error, a folder given to `diff` included,
/// and 4 means a file was rejected or couldn't be read, or the solvers couldn't
/// be found.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" => {
            println!("accord {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [command, path] if command == "check" => check(path, false),
        [command, path, flag] if command == "check" && flag == "--json" => check(path, true),
        [command, path, flag, out] if command == "build" && flag == "--out" => {
            build_modules(path, out)
        }
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

/// A folder's `.accord` files, not its subfolders', in name order, each with
/// its source.
fn folder(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "accord") {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            files.push((name.unwrap_or_default(), std::fs::read_to_string(&path)?));
        }
    }
    files.sort();
    Ok(files)
}

/// A folder's files, or one file with its name, or with no name when
/// `named` is false.
fn sources(target: &Path, named: bool) -> std::io::Result<Vec<(String, String)>> {
    if target.is_dir() {
        return folder(target);
    }
    let name = match target.file_name() {
        Some(name) if named => name.to_string_lossy().into_owned(),
        _ => String::new(),
    };
    Ok(vec![(name, std::fs::read_to_string(target)?)])
}

fn unreadable(path: &str, error: &std::io::Error) -> ExitCode {
    check_error(
        path,
        false,
        Diagnostic {
            code: DiagnosticCode::InputUnavailable,
            file: None,
            line: 0,
            message: error.to_string(),
        },
    )
}

/// Writes a file, or a folder as one program, as TypeScript modules in
/// `out`, once every obligation is proved and every example passes.
fn build_modules(path: &str, out: &str) -> ExitCode {
    let sources = match sources(Path::new(path), true) {
        Ok(sources) => sources,
        Err(error) => return unreadable(path, &error),
    };
    let solvers = match Solvers::from_env() {
        Ok(solvers) => solvers,
        Err(message) => {
            return check_error(
                path,
                false,
                Diagnostic {
                    code: DiagnosticCode::SolverUnavailable,
                    file: None,
                    line: 0,
                    message,
                },
            );
        }
    };
    let built = match build(&sources, &solvers) {
        Ok(built) => built,
        Err(diagnostic) => return check_error(path, false, diagnostic),
    };
    let Some(modules) = built.modules else {
        eprintln!(
            "{path}: nothing written: an obligation isn't proved or an example didn't pass; `accord check {path}` shows which"
        );
        return match built.report.standing {
            Standing::Failed => ExitCode::from(1),
            _ => ExitCode::from(3),
        };
    };
    let written = std::fs::create_dir_all(out).and_then(|()| {
        modules.iter().try_for_each(|module| {
            let file = Path::new(out).join(&module.name);
            std::fs::write(&file, &module.source)?;
            println!("{}", file.display());
            Ok(())
        })
    });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => unreadable(out, &error),
    }
}

/// Checks a file on its own, or a folder as one program: every `.accord` file
/// in it sees the others' declarations.
fn check(path: &str, as_json: bool) -> ExitCode {
    let target = Path::new(path);
    let sources = match sources(target, false) {
        Ok(sources) => sources,
        Err(error) => {
            return check_error(
                path,
                as_json,
                Diagnostic {
                    code: DiagnosticCode::InputUnavailable,
                    file: None,
                    line: 0,
                    message: error.to_string(),
                },
            );
        }
    };
    // The ledger sits beside what's checked, as a cache nobody commits.
    let ledger = if target.is_dir() {
        target.to_path_buf()
    } else {
        target.parent().unwrap_or(Path::new("")).to_path_buf()
    }
    .join(".accord");
    let solvers = match Solvers::from_env() {
        Ok(solvers) => solvers.with_ledger(&ledger).with_mutation(),
        Err(message) => {
            return check_error(
                path,
                as_json,
                Diagnostic {
                    code: DiagnosticCode::SolverUnavailable,
                    file: None,
                    line: 0,
                    message,
                },
            );
        }
    };
    let checked = if target.is_dir() {
        verify_folder(&sources, None, &solvers)
    } else {
        verify(&sources[0].1, &solvers)
    };
    let report = match checked {
        Ok(report) => report,
        Err(diagnostic) => {
            return check_error(path, as_json, diagnostic);
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

fn check_error(path: &str, as_json: bool, diagnostic: Diagnostic) -> ExitCode {
    if as_json {
        println!("{}", receipt::rejection(&diagnostic, path));
    } else {
        let place = diagnostic.file.as_ref().map_or_else(
            || path.to_string(),
            |file| Path::new(path).join(file).display().to_string(),
        );
        if diagnostic.line == 0 {
            eprintln!(
                "{place}: {} [{}]",
                diagnostic.message,
                diagnostic.code.name()
            );
        } else {
            eprintln!(
                "{place}:{}: {} [{}]",
                diagnostic.line,
                diagnostic.message,
                diagnostic.code.name()
            );
        }
    }
    ExitCode::from(4)
}

/// A file's text, or stdin's for `-`.
fn read(path: &str) -> Result<String, ExitCode> {
    if Path::new(path).is_dir() {
        eprintln!("{path}: is a folder; `accord diff` compares two files\n{USAGE}");
        return Err(ExitCode::from(2));
    }
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
    let (old, new) = match read(old_path).and_then(|old| Ok((old, read(new_path)?))) {
        Ok(sides) => sides,
        Err(code) => return code,
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
