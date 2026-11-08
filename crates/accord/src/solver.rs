//! Runs Z3 and cvc5 as separate programs and combines their answers.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::Outcome;

/// The two solvers every obligation goes to.
#[derive(Clone, Debug)]
pub struct Solvers {
    z3: PathBuf,
    cvc5: PathBuf,
    timeout: Duration,
}

/// What one solver said about one obligation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Answer {
    /// The clause can fail; the values are the parameters that make it fail.
    Sat(Vec<(String, String)>),
    /// The clause holds.
    Unsat,
    /// No answer, with the reason.
    Unknown(String),
}

impl Solvers {
    /// Finds Z3 and cvc5 from `ACCORD_Z3` and `ACCORD_CVC5`, or as `z3` and
    /// `cvc5` on the `PATH`.
    pub fn from_env() -> Result<Solvers, String> {
        Ok(Solvers {
            z3: locate("ACCORD_Z3", "z3")?,
            cvc5: locate("ACCORD_CVC5", "cvc5")?,
            timeout: Duration::from_secs(10),
        })
    }

    /// The same solvers with a different time limit per obligation.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Solvers {
        self.timeout = timeout;
        self
    }

    pub(crate) fn decide(&self, script: &str, params: &[String]) -> Outcome {
        let ms = self.timeout.as_millis().max(1);
        let z3 = run(
            &self.z3,
            &["-in", "-smt2", &format!("-t:{ms}")],
            script,
            self.timeout,
            params,
        );
        let cvc5 = run(
            &self.cvc5,
            &["--lang", "smt2", &format!("--tlimit-per={ms}"), "-"],
            script,
            self.timeout,
            params,
        );
        combine(&z3, &cvc5)
    }
}

fn locate(variable: &str, program: &str) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os(variable) {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!("{variable} names no file: {}", path.display()))
        };
    }
    let names = if cfg!(windows) {
        vec![format!("{program}.exe"), program.to_string()]
    } else {
        vec![program.to_string()]
    };
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("`{program}` not found: put it on the PATH or set {variable}"))
}

fn run(
    program: &PathBuf,
    args: &[&str],
    script: &str,
    timeout: Duration,
    params: &[String],
) -> Answer {
    let spawned = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            return Answer::Unknown(format!("{} didn't start: {error}", program.display()));
        }
    };
    if let Some(mut stdin) = child.stdin.take()
        && stdin.write_all(script.as_bytes()).is_err()
    {
        let _ = child.kill();
        return Answer::Unknown(format!("{} didn't take the script", program.display()));
    }
    // The solver's own limit should end it first; this is the backstop.
    let deadline = Instant::now() + timeout + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Answer::Unknown(format!("{} ran out of time", program.display()));
            }
        }
    }
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    parse_answer(&output, params)
}

/// Reads a solver's reply: its first line, then the `get-value` list on `sat`.
pub(crate) fn parse_answer(output: &str, params: &[String]) -> Answer {
    let mut lines = output.lines();
    match lines.next().map(str::trim) {
        Some("unsat") => Answer::Unsat,
        Some("sat") => {
            let rest: String = lines.collect::<Vec<_>>().join(" ");
            let values = params
                .iter()
                .filter_map(|name| value_of(&rest, name).map(|value| (name.clone(), value)))
                .collect::<Vec<_>>();
            if values.len() == params.len() {
                Answer::Sat(values)
            } else {
                Answer::Unknown("sat, but the counterexample couldn't be read".to_string())
            }
        }
        Some("unknown") => Answer::Unknown("the solver answered unknown".to_string()),
        Some("timeout") => Answer::Unknown("the solver ran out of time".to_string()),
        _ => Answer::Unknown("the solver gave no answer".to_string()),
    }
}

fn value_of(text: &str, name: &str) -> Option<String> {
    let start = text.find(&format!("({name} "))? + name.len() + 2;
    let end = text[start..].find(')')? + start;
    Some(text[start..end].trim().to_string())
}

/// Two answers become one outcome. Only agreement proves or refutes.
pub(crate) fn combine(z3: &Answer, cvc5: &Answer) -> Outcome {
    match (z3, cvc5) {
        (Answer::Unsat, Answer::Unsat) => Outcome::Proved,
        (Answer::Sat(values), Answer::Sat(_)) => Outcome::Refuted {
            counterexample: values.clone(),
        },
        (Answer::Sat(_), Answer::Unsat) | (Answer::Unsat, Answer::Sat(_)) => Outcome::Undecided {
            reason: "the solvers disagree: one proved it and the other refuted it".to_string(),
        },
        (Answer::Unknown(reason), _) => Outcome::Undecided {
            reason: format!("Z3: {reason}"),
        },
        (_, Answer::Unknown(reason)) => Outcome::Undecided {
            reason: format!("cvc5: {reason}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, Solvers, combine, parse_answer};
    use crate::Outcome;
    use std::path::PathBuf;
    use std::time::Duration;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn only_two_unsat_answers_prove() {
        assert_eq!(combine(&Answer::Unsat, &Answer::Unsat), Outcome::Proved);
        let unknown = Answer::Unknown("ran out of time".to_string());
        assert!(matches!(
            combine(&Answer::Unsat, &unknown),
            Outcome::Undecided { .. }
        ));
        assert!(matches!(
            combine(&unknown, &Answer::Unsat),
            Outcome::Undecided { .. }
        ));
    }

    #[test]
    fn disagreement_is_undecided() {
        let sat = Answer::Sat(vec![("a".to_string(), "true".to_string())]);
        assert!(matches!(
            combine(&sat, &Answer::Unsat),
            Outcome::Undecided { .. }
        ));
        assert!(matches!(
            combine(&Answer::Unsat, &sat),
            Outcome::Undecided { .. }
        ));
    }

    #[test]
    fn a_timeout_or_garbage_is_unknown() {
        assert!(matches!(parse_answer("timeout\n", &[]), Answer::Unknown(_)));
        assert!(matches!(parse_answer("unknown\n", &[]), Answer::Unknown(_)));
        assert!(matches!(
            parse_answer("(error \"bad\")\n", &[]),
            Answer::Unknown(_)
        ));
        assert!(matches!(parse_answer("", &[]), Answer::Unknown(_)));
    }

    #[test]
    fn sat_needs_a_value_for_every_parameter() {
        let both = parse_answer("sat\n((a true)\n (b false))\n", &names(&["a", "b"]));
        assert_eq!(
            both,
            Answer::Sat(vec![
                ("a".to_string(), "true".to_string()),
                ("b".to_string(), "false".to_string())
            ])
        );
        assert!(matches!(
            parse_answer("sat\n((a true))\n", &names(&["a", "b"])),
            Answer::Unknown(_)
        ));
    }

    #[test]
    fn a_missing_solver_never_proves() {
        let solvers = Solvers {
            z3: PathBuf::from("no-such-solver-z3"),
            cvc5: PathBuf::from("no-such-solver-cvc5"),
            timeout: Duration::from_secs(1),
        };
        let script = "(set-logic QF_UF)\n(assert false)\n(check-sat)\n";
        assert!(matches!(
            solvers.decide(script, &[]),
            Outcome::Undecided { .. }
        ));
    }
}
