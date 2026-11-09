//! Accord's compiler: it parses a source file, checks it, turns each
//! specification into proof obligations and has two SMT solvers decide them.

mod check;
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
}

/// Parses and checks `source`, then decides every obligation in it.
///
/// Returns one verdict per obligation, in source order: each `ensures`,
/// then for a decision table its coverage and each row's reachability. Or
/// the diagnostic that rejected the file.
pub fn verify(source: &str, solvers: &Solvers) -> Result<Vec<Verdict>, Diagnostic> {
    let mut program = syntax::parse(source)?;
    check::check(&mut program)?;
    Ok(obligation::obligations(&program)
        .into_iter()
        .map(|o| Verdict {
            outcome: solvers.decide(&o.script, &o.params, o.polarity),
            function: o.function,
            clause: o.clause,
            line: o.line,
        })
        .collect())
}
