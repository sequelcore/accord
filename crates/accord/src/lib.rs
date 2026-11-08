//! Accord's compiler: it parses a source file, checks it, turns each
//! specification into proof obligations and has two SMT solvers decide them.

mod check;
mod obligation;
mod solver;
mod syntax;

pub use solver::Solvers;

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
    Proved,
    /// Both solvers found inputs on which it fails; these are one solver's.
    Refuted {
        /// Each parameter's name and the value that makes the obligation fail.
        counterexample: Vec<(String, String)>,
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
/// Returns one verdict per `ensures`, in source order, or the diagnostic
/// that rejected the file.
pub fn verify(source: &str, solvers: &Solvers) -> Result<Vec<Verdict>, Diagnostic> {
    let functions = syntax::parse(source)?;
    check::check(&functions)?;
    Ok(obligation::obligations(&functions)
        .into_iter()
        .map(|o| Verdict {
            outcome: solvers.decide(&o.script, &o.params),
            function: o.function,
            clause: o.clause,
            line: o.line,
        })
        .collect())
}
