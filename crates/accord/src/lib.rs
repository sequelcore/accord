//! Accord's compiler: it parses a source file, checks it, turns each
//! specification into proof obligations, has two SMT solvers decide them and
//! runs its examples.

mod check;
mod identity;
mod interpret;
mod obligation;
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

/// Everything one check of a file established.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// One verdict per obligation, in source order.
    pub verdicts: Vec<Verdict>,
    /// One run per `example`, in source order.
    pub examples: Vec<ExampleRun>,
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
            let outcome = solvers.decide(&o.script, &o.params, o.polarity, |model| {
                interpret::replay(&o, model)
            });
            verdicts.push(Verdict {
                outcome: unit.source_outcome(outcome),
                function: unit.source.name.clone(),
                key: identity::hash(&o.script),
                definition: unit.hash.clone(),
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
        .collect();
    Ok(Report { verdicts, examples })
}
