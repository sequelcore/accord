//! Runs Z3 and cvc5 as separate programs and combines their answers.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

/// What one solver said about one obligation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Answer {
    /// The script's assertions can hold, with the parameters' values.
    Sat(Vec<(String, String)>),
    /// They can't.
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
        if let Some(outcome) = self.ledger.as_ref().and_then(|l| l.get(script)) {
            return (outcome, true);
        }
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
        let outcome = confirm(combine(&z3, &cvc5, polarity), &z3, &cvc5, replay);
        if let Some(ledger) = &self.ledger {
            ledger.put(script, &outcome);
        }
        (outcome, false)
    }
}

/// An outcome that rests on models stands only if `replay` reproduces each
/// solver's model; otherwise it is undecided.
pub(crate) fn confirm(
    outcome: Outcome,
    z3: &Answer,
    cvc5: &Answer,
    replay: impl Fn(&Inputs) -> Result<(), String>,
) -> Outcome {
    let kind = match &outcome {
        Outcome::Proved { witness: Some(_) } => "witness",
        Outcome::Refuted {
            evidence: Refutation::Counterexample(_),
        } => "counterexample",
        _ => return outcome,
    };
    for (solver, answer) in [("Z3", z3), ("cvc5", cvc5)] {
        if let Answer::Sat(model) = answer
            && let Err(why) = replay(model)
        {
            return Outcome::Undecided {
                reason: format!("{solver}'s {kind} doesn't reproduce: {why}"),
            };
        }
    }
    outcome
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
        Some("unknown") => {
            let rest: String = lines.collect::<Vec<_>>().join(" ");
            let reason = rest
                .split_once("(:reason-unknown ")
                .and_then(|(_, r)| r.split(')').next())
                .map(|r| r.trim().trim_matches('"'));
            Answer::Unknown(match reason {
                Some("timeout") => "the solver ran out of time".to_string(),
                Some(reason) if !reason.is_empty() => {
                    format!("the solver answered unknown ({reason})")
                }
                _ => "the solver answered unknown".to_string(),
            })
        }
        Some("timeout") => Answer::Unknown("the solver ran out of time".to_string()),
        _ => Answer::Unknown("the solver gave no answer".to_string()),
    }
}

/// One SMT-LIB value as `get-value` prints it.
enum Sexp {
    Atom(String),
    /// A string literal's characters, decoded.
    Str(String),
    List(Vec<Sexp>),
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn parse_sexp(chars: &mut Chars) -> Option<Sexp> {
    while chars.peek()?.is_whitespace() {
        chars.next();
    }
    match chars.next()? {
        '(' => {
            let mut items = Vec::new();
            loop {
                while chars.peek()?.is_whitespace() {
                    chars.next();
                }
                if chars.peek() == Some(&')') {
                    chars.next();
                    return Some(Sexp::List(items));
                }
                items.push(parse_sexp(chars)?);
            }
        }
        '"' => read_string(chars).map(Sexp::Str),
        first => {
            let mut atom = String::from(first);
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '(' || c == ')' {
                    break;
                }
                atom.push(c);
                chars.next();
            }
            Some(Sexp::Atom(atom))
        }
    }
}

/// An SMT-LIB string literal's characters, after its opening quote: `""` is
/// a quote, and `\u{...}` or `\uXXXX` a character. Z3 prints a backslash
/// raw, so a text that itself contains `\u{...}` reads back as the character
/// it spells; the interpreter's replay then rejects the model, and the
/// obligation is undecided, never wrongly decided.
fn read_string(chars: &mut Chars) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                out.push('"');
            }
            '"' => return Some(out),
            '\\' if chars.peek() == Some(&'u') => {
                chars.next();
                let hex: String = if chars.peek() == Some(&'{') {
                    chars.next();
                    chars.by_ref().take_while(|c| *c != '}').collect()
                } else {
                    chars.by_ref().take(4).collect()
                };
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            c => out.push(c),
        }
    }
}

/// A sequence value's elements, in Accord source: `(seq.unit x)`,
/// `(seq.++ ...)` and `(as seq.empty ...)`.
fn elements(value: &Sexp) -> Option<Vec<String>> {
    let Sexp::List(items) = value else {
        return None;
    };
    match items.as_slice() {
        [Sexp::Atom(f), x] if f == "seq.unit" => Some(vec![source(x)?]),
        [Sexp::Atom(f), parts @ ..] if f == "seq.++" => {
            let mut out = Vec::new();
            for part in parts {
                out.extend(elements(part)?);
            }
            Some(out)
        }
        [Sexp::Atom(a), Sexp::Atom(e), _] if a == "as" && e == "seq.empty" => Some(Vec::new()),
        _ => None,
    }
}

/// A value in Accord source: a symbol or number as printed, `(- n)` as `-n`,
/// a string in quotes, a sequence as a list literal.
fn source(value: &Sexp) -> Option<String> {
    match value {
        Sexp::Atom(atom) => Some(atom.clone()),
        Sexp::Str(text) => Some(format!("\"{text}\"")),
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Atom(minus), Sexp::Atom(n)] if minus == "-" => Some(format!("-{n}")),
            _ => elements(value).map(|items| format!("[{}]", items.join(", "))),
        },
    }
}

/// The value `get-value` gives `name`, in Accord source.
fn value_of(text: &str, name: &str) -> Option<String> {
    let start = text.find(&format!("({name} "))? + name.len() + 2;
    source(&parse_sexp(&mut text[start..].chars().peekable())?)
}

/// Two answers become one outcome. Only agreement proves or refutes. A
/// universal obligation's script asserts that it fails, so `unsat` proves it;
/// an existential one's asserts the input it needs, so `sat` proves it.
pub(crate) fn combine(z3: &Answer, cvc5: &Answer, polarity: Polarity) -> Outcome {
    match (z3, cvc5, polarity) {
        (Answer::Unsat, Answer::Unsat, Polarity::Universal) => Outcome::Proved { witness: None },
        (Answer::Unsat, Answer::Unsat, Polarity::Existential) => Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        },
        (Answer::Sat(values), Answer::Sat(_), Polarity::Universal) => Outcome::Refuted {
            evidence: Refutation::Counterexample(values.clone()),
        },
        (Answer::Sat(values), Answer::Sat(_), Polarity::Existential) => Outcome::Proved {
            witness: Some(values.clone()),
        },
        (Answer::Sat(_), Answer::Unsat, _) | (Answer::Unsat, Answer::Sat(_), _) => {
            Outcome::Undecided {
                reason: "the solvers disagree: one found an input and the other proved none exists"
                    .to_string(),
            }
        }
        (Answer::Unknown(reason), _, _) => Outcome::Undecided {
            reason: format!("Z3: {reason}"),
        },
        (_, Answer::Unknown(reason), _) => Outcome::Undecided {
            reason: format!("cvc5: {reason}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, Solvers, combine, confirm, parse_answer};
    use crate::obligation::Polarity::{Existential, Universal};
    use crate::{Outcome, Refutation};
    use std::path::PathBuf;
    use std::time::Duration;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn only_two_unsat_answers_prove_a_universal_obligation() {
        assert_eq!(
            combine(&Answer::Unsat, &Answer::Unsat, Universal),
            Outcome::Proved { witness: None }
        );
        let unknown = Answer::Unknown("ran out of time".to_string());
        for polarity in [Universal, Existential] {
            assert!(matches!(
                combine(&Answer::Unsat, &unknown, polarity),
                Outcome::Undecided { .. }
            ));
            assert!(matches!(
                combine(&unknown, &Answer::Unsat, polarity),
                Outcome::Undecided { .. }
            ));
        }
    }

    #[test]
    fn an_existential_obligation_is_proved_by_a_witness_and_refuted_by_two_unsat() {
        let sat = Answer::Sat(vec![("a".to_string(), "true".to_string())]);
        assert_eq!(
            combine(&sat, &sat, Existential),
            Outcome::Proved {
                witness: Some(vec![("a".to_string(), "true".to_string())])
            }
        );
        assert_eq!(
            combine(&Answer::Unsat, &Answer::Unsat, Existential),
            Outcome::Refuted {
                evidence: Refutation::NoSuchInput
            }
        );
        let unknown = Answer::Unknown("ran out of time".to_string());
        assert!(matches!(
            combine(&sat, &unknown, Existential),
            Outcome::Undecided { .. }
        ));
    }

    #[test]
    fn disagreement_is_undecided() {
        let sat = Answer::Sat(vec![("a".to_string(), "true".to_string())]);
        for polarity in [Universal, Existential] {
            assert!(matches!(
                combine(&sat, &Answer::Unsat, polarity),
                Outcome::Undecided { .. }
            ));
            assert!(matches!(
                combine(&Answer::Unsat, &sat, polarity),
                Outcome::Undecided { .. }
            ));
        }
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
    fn a_model_that_does_not_replay_leaves_the_obligation_undecided() {
        let a = |v: &str| Answer::Sat(vec![("a".to_string(), v.to_string())]);
        let (good, bad) = (a("1"), a("2"));
        let replay = |model: &crate::Inputs| match model[0].1.as_str() {
            "1" => Ok(()),
            _ => Err("no".to_string()),
        };
        for polarity in [Universal, Existential] {
            let agreed = combine(&good, &good, polarity);
            assert_eq!(confirm(agreed.clone(), &good, &good, replay), agreed);
            let undecided = |z3: &Answer, cvc5: &Answer| {
                matches!(
                    confirm(combine(z3, cvc5, polarity), z3, cvc5, replay),
                    Outcome::Undecided { .. }
                )
            };
            assert!(undecided(&bad, &good), "Z3's model is checked");
            assert!(undecided(&good, &bad), "cvc5's model is checked too");
        }
        // An outcome that rests on `unsat` has no model to replay.
        let proved = combine(&Answer::Unsat, &Answer::Unsat, Universal);
        assert_eq!(
            confirm(proved.clone(), &Answer::Unsat, &Answer::Unsat, |_| Err(
                "never called".to_string()
            )),
            proved
        );
    }

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
    fn an_unknown_answer_says_when_the_solver_ran_out_of_time() {
        let z3 = "unknown\n(error \"model is not available\")\n(:reason-unknown \"timeout\")\n";
        let cvc5 = "unknown\n((a 0))\n(:reason-unknown timeout)\n";
        for output in [z3, cvc5] {
            assert_eq!(
                parse_answer(output, &names(&["a"])),
                Answer::Unknown("the solver ran out of time".to_string())
            );
        }
        assert_eq!(
            parse_answer("unknown\n(:reason-unknown incomplete)\n", &[]),
            Answer::Unknown("the solver answered unknown (incomplete)".to_string())
        );
    }

    #[test]
    fn text_values_read_as_source() {
        let both = parse_answer(
            "sat\n((a \"caf\\u{e9}\")\n (b \"say \"\"hi\"\"\"))\n",
            &names(&["a", "b"]),
        );
        assert_eq!(
            both,
            Answer::Sat(vec![
                ("a".to_string(), "\"café\"".to_string()),
                ("b".to_string(), "\"say \"hi\"\"".to_string())
            ])
        );
    }

    #[test]
    fn a_raw_backslash_from_z3_reads_as_itself() {
        // Z3 prints a backslash raw where cvc5 writes `\u{5c}`; both read alike.
        for output in ["sat\n((a \"x \\\"))\n", "sat\n((a \"x \\u{5c}\"))\n"] {
            assert_eq!(
                parse_answer(output, &names(&["a"])),
                Answer::Sat(vec![("a".to_string(), "\"x \\\"".to_string())])
            );
        }
    }

    #[test]
    fn sequences_read_as_list_literals() {
        // Both solvers' forms, with a `)` inside a text and an empty list.
        let output = "sat\n((s (seq.++ (seq.unit \"x)\") (seq.unit \"caf\\u{e9}\")))\n (c (seq.unit E0.v1))\n (e (as seq.empty (Seq String))))\n";
        assert_eq!(
            parse_answer(output, &names(&["s", "c", "e"])),
            Answer::Sat(vec![
                ("s".to_string(), "[\"x)\", \"café\"]".to_string()),
                ("c".to_string(), "[E0.v1]".to_string()),
                ("e".to_string(), "[]".to_string())
            ])
        );
    }

    #[test]
    fn negative_values_read_as_source() {
        assert_eq!(
            parse_answer("sat\n((a (- 2147483648))\n (b 7))\n", &names(&["a", "b"])),
            Answer::Sat(vec![
                ("a".to_string(), "-2147483648".to_string()),
                ("b".to_string(), "7".to_string())
            ])
        );
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
