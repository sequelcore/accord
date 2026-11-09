//! Accord's compiler: it parses a source file, checks it, turns each
//! specification into proof obligations, has two SMT solvers decide them and
//! runs its examples.

mod check;
mod identity;
mod interpret;
mod ledger;
mod obligation;
pub mod receipt;
mod solver;
mod syntax;

pub use solver::Solvers;

/// Parameter names with the values a solver gave them, in source terms.
pub type Inputs = Vec<(String, String)>;

/// A reason a file was rejected before any obligation was generated.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The 1-based line the problem is on.
    pub line: usize,
    /// What is wrong, for a person to read.
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            line,
            message: message.into(),
        }
    }
}

/// What the solvers established about one obligation.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Both solvers proved it.
    Proved {
        /// For an obligation that some input exists, such as one reaching a
        /// decision table's row: one solver's input that does.
        witness: Option<Inputs>,
    },
    /// Both solvers established that it fails.
    Refuted {
        /// How it fails.
        evidence: Refutation,
    },
    /// No proof and no agreed counterexample.
    Undecided {
        /// Why: a time limit, an unknown answer, or the solvers disagreeing.
        reason: String,
    },
    /// It uses something the compiler can't check yet.
    Unsupported {
        /// What isn't supported.
        reason: String,
    },
}

/// What shows that an obligation fails.
#[derive(Clone, Debug, PartialEq)]
pub enum Refutation {
    /// Inputs on which it fails, from one solver.
    Counterexample(Inputs),
    /// It needs some input, and both solvers proved that none exists.
    NoSuchInput,
}

/// The outcome of one obligation, with where it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    /// The function the obligation belongs to.
    pub function: String,
    /// The clause as written in the source.
    pub clause: String,
    /// The clause's 1-based line.
    pub line: usize,
    /// What the solvers established.
    pub outcome: Outcome,
    /// The hash of exactly what was checked: the obligation's script, written
    /// from the normalized function, so no rename changes it.
    pub key: String,
    /// The hash of the normalized function the obligation belongs to.
    pub definition: String,
    /// Whether the outcome came from the ledger rather than the solvers.
    pub reused: bool,
}

/// Whether an example gave its result when run.
#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    /// The call gave the expected result.
    Passed,
    /// It didn't, broke a `requires`, overflowed or reached no row.
    Failed {
        /// What went wrong, for a person to read.
        reason: String,
    },
}

/// The run of one `example`, with where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct ExampleRun {
    /// The function the example calls.
    pub function: String,
    /// The example's first line as written.
    pub text: String,
    /// The example's 1-based line.
    pub line: usize,
    /// How the run went.
    pub run: Run,
}

/// What a definition's evidence, or a file's, adds up to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Standing {
    /// Every obligation proved and every example passed.
    Verified,
    /// Nothing failed, but something keeps it from verified: an undecided or
    /// unsupported obligation, or, for a file, a definition with no evidence.
    Incomplete,
    /// An obligation was refuted or an example failed.
    Failed,
    /// There is no evidence: nothing was checked.
    Unverified,
}

/// One function's identity and standing.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    /// The function's name.
    pub name: String,
    /// The hash of the normalized function.
    pub hash: String,
    /// What its obligations and examples add up to.
    pub standing: Standing,
}

/// Everything one check of a file established.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// One verdict per obligation, in source order.
    pub verdicts: Vec<Verdict>,
    /// One run per `example`, in source order.
    pub examples: Vec<ExampleRun>,
    /// Every function, in source order.
    pub definitions: Vec<Definition>,
    /// The file's standing.
    pub standing: Standing,
}

/// A definition's standing from its own evidence.
fn standing_of<'a>(
    verdicts: impl Iterator<Item = &'a Verdict>,
    examples: impl Iterator<Item = &'a ExampleRun>,
) -> Standing {
    let (mut any, mut failed, mut open) = (false, false, false);
    for verdict in verdicts {
        any = true;
        match verdict.outcome {
            Outcome::Proved { .. } => {}
            Outcome::Refuted { .. } => failed = true,
            Outcome::Undecided { .. } | Outcome::Unsupported { .. } => open = true,
        }
    }
    for example in examples {
        any = true;
        failed |= example.run != Run::Passed;
    }
    match (failed, open, any) {
        (true, _, _) => Standing::Failed,
        (false, true, _) => Standing::Incomplete,
        (false, false, true) => Standing::Verified,
        (false, false, false) => Standing::Unverified,
    }
}

/// A file's standing: failed if any definition failed, verified only if
/// every one is, unverified if none has evidence, and incomplete otherwise.
fn file_standing(definitions: &[Definition]) -> Standing {
    let all = |s: Standing| definitions.iter().all(|d| d.standing == s);
    if definitions.iter().any(|d| d.standing == Standing::Failed) {
        Standing::Failed
    } else if !definitions.is_empty() && all(Standing::Verified) {
        Standing::Verified
    } else if all(Standing::Unverified) {
        Standing::Unverified
    } else {
        Standing::Incomplete
    }
}

/// Parses and checks `source`, decides every obligation in it and runs every
/// example.
///
/// Verdicts are in source order: per function, each `ensures`, a decision
/// table's coverage and each row's reachability, and each arithmetic
/// operation's range. Or the diagnostic that rejected the file.
pub fn verify(source: &str, solvers: &Solvers) -> Result<Report, Diagnostic> {
    let mut program = syntax::parse(source)?;
    check::check(&mut program)?;
    let units = identity::units(&program);
    let mut verdicts = Vec::new();
    for unit in &units {
        for o in obligation::obligations(&unit.program) {
            let (outcome, reused) = solvers.decide(&o.script, &o.params, o.polarity, |model| {
                interpret::replay(&o, model)
            });
            verdicts.push(Verdict {
                outcome: unit.source_outcome(outcome),
                function: unit.source.name.clone(),
                key: identity::hash(&o.script),
                definition: unit.hash.clone(),
                reused,
                clause: o.clause,
                line: o.line,
            });
        }
    }
    let examples = program
        .examples
        .iter()
        .filter_map(|example| {
            let function = program
                .functions
                .iter()
                .find(|f| f.name == example.function)?;
            Some(ExampleRun {
                function: function.name.clone(),
                text: example.text.clone(),
                line: example.line,
                run: match interpret::run(example, function) {
                    Ok(()) => Run::Passed,
                    Err(reason) => Run::Failed { reason },
                },
            })
        })
        .collect::<Vec<_>>();
    let definitions: Vec<Definition> = units
        .iter()
        .map(|unit| Definition {
            name: unit.source.name.clone(),
            hash: unit.hash.clone(),
            standing: standing_of(
                verdicts.iter().filter(|v| v.function == unit.source.name),
                examples.iter().filter(|e| e.function == unit.source.name),
            ),
        })
        .collect();
    let standing = file_standing(&definitions);
    Ok(Report {
        verdicts,
        examples,
        definitions,
        standing,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Definition, ExampleRun, Outcome, Run, Standing, Verdict, file_standing, standing_of,
    };

    fn verdict(outcome: Outcome) -> Verdict {
        Verdict {
            function: "f".to_string(),
            clause: "ensures result".to_string(),
            line: 1,
            outcome,
            key: String::new(),
            definition: String::new(),
            reused: false,
        }
    }

    fn example(run: Run) -> ExampleRun {
        ExampleRun {
            function: "f".to_string(),
            text: "example f() == true".to_string(),
            line: 2,
            run,
        }
    }

    #[test]
    fn a_definition_stands_on_its_own_evidence() {
        let proved = verdict(Outcome::Proved { witness: None });
        let refuted = verdict(Outcome::Refuted {
            evidence: crate::Refutation::NoSuchInput,
        });
        let undecided = verdict(Outcome::Undecided {
            reason: "timeout".to_string(),
        });
        let failed = example(Run::Failed {
            reason: "no".to_string(),
        });
        let of =
            |v: &[&Verdict], e: &[&ExampleRun]| standing_of(v.iter().copied(), e.iter().copied());
        assert_eq!(of(&[&proved], &[&example(Run::Passed)]), Standing::Verified);
        assert_eq!(of(&[&proved, &undecided], &[]), Standing::Incomplete);
        assert_eq!(of(&[&undecided, &refuted], &[]), Standing::Failed);
        assert_eq!(of(&[&proved], &[&failed]), Standing::Failed);
        assert_eq!(of(&[], &[]), Standing::Unverified);
    }

    #[test]
    fn a_file_is_verified_only_when_every_definition_is() {
        let with = |standings: &[Standing]| {
            let definitions: Vec<Definition> = standings
                .iter()
                .map(|&standing| Definition {
                    name: "f".to_string(),
                    hash: String::new(),
                    standing,
                })
                .collect();
            file_standing(&definitions)
        };
        use Standing::{Failed, Incomplete, Unverified, Verified};
        assert_eq!(with(&[Verified, Verified]), Verified);
        assert_eq!(with(&[Verified, Unverified]), Incomplete);
        assert_eq!(with(&[Verified, Incomplete]), Incomplete);
        assert_eq!(with(&[Incomplete, Failed]), Failed);
        assert_eq!(with(&[Unverified]), Unverified);
        assert_eq!(with(&[]), Unverified);
    }
}
