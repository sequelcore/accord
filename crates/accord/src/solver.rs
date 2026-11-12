//! Runs Z3 and cvc5 as separate programs and combines their answers.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod combine;
mod reply;

use combine::{combine, confirm};
use reply::{Answer, parse_answer};

use crate::interpret::Replay;
use crate::ledger::Ledger;
use crate::obligation::Polarity;
use crate::{Inputs, Outcome, Refutation};

/// The two solvers every obligation goes to.
#[derive(Clone, Debug)]
pub struct Solvers {
    z3: PathBuf,
    cvc5: PathBuf,
    timeout: Duration,
    ledger: Option<Ledger>,
    mutate: bool,
}

impl Solvers {
    /// Finds Z3 and cvc5 from `ACCORD_Z3` and `ACCORD_CVC5`, or as `z3` and
    /// `cvc5` on the `PATH`.
    pub fn from_env() -> Result<Solvers, String> {
        Ok(Solvers {
            z3: locate("ACCORD_Z3", "z3")?,
            cvc5: locate("ACCORD_CVC5", "cvc5")?,
            timeout: Duration::from_secs(10),
            ledger: None,
            mutate: false,
        })
    }

    /// The same solvers with a different time limit per obligation.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Solvers {
        self.timeout = timeout;
        self
    }

    /// The same solvers, reusing and storing evidence in a ledger in `dir`.
    /// If the solvers' versions can't be read, nothing is reused.
    #[must_use]
    pub fn with_ledger(mut self, dir: &std::path::Path) -> Solvers {
        self.ledger = Ledger::open(dir, &self.z3, &self.cvc5);
        self
    }

    /// The same solvers, also judging each function's contract against
    /// mutants of its body.
    #[must_use]
    pub fn with_mutation(mut self) -> Solvers {
        self.mutate = true;
        self
    }

    pub(crate) fn mutates(&self) -> bool {
        self.mutate
    }

    /// Both solvers' answers, combined, and whether they came from the
    /// ledger. A counterexample or witness counts
    /// only if `replay` reproduces every model the solvers gave.
    pub(crate) fn decide(
        &self,
        script: &str,
        params: &[String],
        polarity: Polarity,
        replay: impl Fn(&Inputs) -> Result<(), String>,
    ) -> (Outcome, bool) {
        self.decide_calls(script, "", params, polarity, |model| {
            replay(model).map(|()| Replay::Runs)
        })
    }

    /// As [`Solvers::decide`], for a script about code that calls other
    /// functions, whose bodies `callees` identifies. The script holds only
    /// their contracts, so an outcome that rests on `unsat` is stored by the
    /// script alone; one that rests on a replayed model also depends on the
    /// bodies, and is stored with them.
    pub(crate) fn decide_calls(
        &self,
        script: &str,
        callees: &str,
        params: &[String],
        polarity: Polarity,
        replay: impl Fn(&Inputs) -> Result<Replay, String>,
    ) -> (Outcome, bool) {
        let with_bodies = format!("{script}; callees {callees}\n");
        let keys: Vec<&str> = if callees.is_empty() {
            vec![script]
        } else {
            vec![script, &with_bodies]
        };
        for key in &keys {
            if let Some(outcome) = self.ledger.as_ref().and_then(|l| l.get(key)) {
                return (outcome, true);
            }
        }
        let ms = self.timeout.as_millis().max(1);
        let z3 = run(
            &self.z3,
            &["-in", "-smt2", &format!("-t:{ms}")],
            script,
            self.timeout,
            params,
        );
        // Without model-based quantifier instantiation, cvc5 finds no input
        // for a claim with a quantifier (docs/design/measurements).
        let cvc5 = run(
            &self.cvc5,
            &[
                "--lang",
                "smt2",
                "--mbqi",
                &format!("--tlimit-per={ms}"),
                "-",
            ],
            script,
            self.timeout,
            params,
        );
        let outcome = confirm(combine(&z3, &cvc5, polarity), &z3, &cvc5, replay);
        if let Some(ledger) = &self.ledger {
            let rests_on_a_model = matches!(
                outcome,
                Outcome::Proved { witness: Some(_) }
                    | Outcome::Refuted {
                        evidence: Refutation::Counterexample(_)
                    }
                    | Outcome::WeakCallee { .. }
            );
            let key = if rests_on_a_model {
                keys[keys.len() - 1]
            } else {
                script
            };
            ledger.put(key, &outcome);
        }
        (outcome, false)
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

#[cfg(test)]
mod tests {
    use super::{Answer, Solvers};
    use crate::Outcome;
    use crate::obligation::Polarity::{Existential, Universal};
    use std::path::PathBuf;
    use std::time::Duration;

    /// A stand-in for a solver that ignores its own time limit. It runs only
    /// when `a_solver_that_never_answers_is_killed` starts it.
    #[test]
    #[ignore = "started as a child process by a_solver_that_never_answers_is_killed"]
    fn never_answers() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn a_solver_that_never_answers_is_killed() {
        let this = std::env::current_exe().expect("the test binary's path");
        let args = ["--ignored", "--exact", "solver::tests::never_answers"];
        let timeout = Duration::from_millis(100);
        let start = std::time::Instant::now();
        let answer = super::run(&this, &args, "(check-sat)\n", timeout, &[]);
        let elapsed = start.elapsed();
        assert_eq!(
            answer,
            Answer::Unknown(format!("{} ran out of time", this.display()))
        );
        // The backstop waits five seconds past the limit, then kills it.
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    }

    #[test]
    fn a_missing_solver_never_decides() {
        let solvers = Solvers {
            z3: PathBuf::from("no-such-solver-z3"),
            cvc5: PathBuf::from("no-such-solver-cvc5"),
            timeout: Duration::from_secs(1),
            ledger: None,
            mutate: false,
        };
        let script = "(set-logic QF_UF)\n(assert false)\n(check-sat)\n";
        for polarity in [Universal, Existential] {
            assert!(matches!(
                solvers.decide(script, &[], polarity, |_| Ok(())).0,
                Outcome::Undecided { .. }
            ));
        }
    }
}
