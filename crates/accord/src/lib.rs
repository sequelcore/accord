//! Accord's compiler: it parses a source file, checks it, turns each
//! specification into proof obligations, has two SMT solvers decide them and
//! runs its examples.

mod check;
mod diagnostic;
pub mod diff;
mod evidence;
mod identity;
mod interpret;
mod ledger;
mod mutation;
mod obligation;
mod openness;
pub mod receipt;
mod solver;
mod syntax;
mod typescript;

use evidence::report;

pub use diagnostic::DiagnosticCode;
pub use mutation::{Strength, Survivor};
pub use openness::{Determinacy, Openness};
pub use solver::Solvers;
pub use syntax::{Type, Width};
pub use typescript::{Build, Module, build};

/// Parameter names with the values a solver gave them, in source terms.
pub type Inputs = Vec<(String, String)>;

/// A reason a file was rejected before any obligation was generated.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The stable machine condition.
    pub code: DiagnosticCode,
    /// The file the problem is in, for a folder of files; `None` for one file.
    pub file: Option<String>,
    /// The 1-based line the problem is on.
    pub line: usize,
    /// What is wrong, for a person to read.
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            code: DiagnosticCode::SourceRejected,
            file: None,
            line,
            message: message.into(),
        }
    }

    /// The same diagnostic about `file`, unless it already names one or
    /// `file` is empty, as it is when a single file was checked.
    fn in_file(mut self, file: &str) -> Diagnostic {
        if !file.is_empty() {
            self.file.get_or_insert_with(|| file.to_string());
        }
        self
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
    /// Not proved from the contracts of the functions it calls, though the
    /// code may be right: the solvers' counterexample fails only when a call
    /// takes a result its callee's contract allows and its body never gives.
    /// Strengthening that contract can settle it.
    WeakCallee {
        /// The counterexample's inputs.
        inputs: Inputs,
        /// Each call as written, with the result the counterexample gives it.
        calls: Inputs,
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

/// The property an obligation checks, independent of its wording.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObligationKind {
    /// The implementation meets an ensures clause.
    Postcondition,
    /// Every admitted input reaches a decision row.
    Coverage,
    /// Some admitted input reaches this row first.
    RowReachability,
    /// An executable arithmetic operation stays within its type.
    Overflow,
    /// A call meets its callee's `requires`.
    Precondition,
    /// An index is within its list.
    Bounds,
}

impl ObligationKind {
    /// Stable machine name of the property.
    pub fn name(self) -> &'static str {
        match self {
            Self::Postcondition => "postcondition",
            Self::Coverage => "coverage",
            Self::RowReachability => "row_reachability",
            Self::Overflow => "overflow",
            Self::Precondition => "precondition",
            Self::Bounds => "bounds",
        }
    }
}

/// The outcome of one obligation, with where it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    /// What was checked; row reachability is existential, all others universal.
    pub kind: ObligationKind,
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
    /// unsupported obligation, an open case, or, for a file, a definition
    /// with no evidence.
    Incomplete,
    /// An obligation was refuted or an example failed.
    Failed,
    /// There is no evidence: nothing was checked.
    Unverified,
}

/// One function's identity, unchecked intent, and standing.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    /// The function's name.
    pub name: String,
    /// Its declared purpose in unchecked natural language, when present.
    pub intent: Option<String>,
    /// The declaration's 1-based line.
    pub line: usize,
    /// Parameter names and types, in declaration order.
    pub params: Vec<(String, Type)>,
    /// The result's type.
    pub result_type: Type,
    /// Source lines and text of every specification clause, including requires.
    pub specification: Vec<(usize, String)>,
    /// Enums used by this function, with their source variants.
    pub enums: Vec<(String, Vec<String>)>,
    /// The file it's declared in, when a folder was checked.
    pub file: Option<String>,
    /// The hash of the normalized function.
    pub hash: String,
    /// What its obligations and examples add up to, and those of every
    /// function it calls: it is verified only when they all are.
    pub standing: Standing,
    /// The functions it calls, directly or not, that aren't verified; its
    /// own proofs assume their contracts.
    pub unverified_callees: Vec<String>,
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
    /// Each contract's strength against mutants, when the solvers were asked
    /// for it; it doesn't change standing.
    pub strength: Vec<Strength>,
    /// Whether each contract with an `ensures` determines its result.
    pub openness: Vec<Determinacy>,
}

/// Parses and checks `source`, decides every obligation in it and runs every
/// example.
///
/// Verdicts are in source order: per function, each `ensures`, a decision
/// table's coverage and each row's reachability, and each arithmetic
/// operation's range. Or the diagnostic that rejected the file.
pub fn verify(source: &str, solvers: &Solvers) -> Result<Report, Diagnostic> {
    let mut program = syntax::parse(source)?;
    check::check(&mut program, &[])?;
    Ok(report(&program, None, &[], solvers))
}

/// Parses and checks a folder's files as one program, each a file's name and
/// its source, and decides what `only` declares, or every file's when it's
/// `None`. Every file's declarations are in view of every other's; a name
/// declared twice is an error. Or the diagnostic that rejected a file, naming
/// it.
pub fn verify_folder(
    files: &[(String, String)],
    only: Option<&str>,
    solvers: &Solvers,
) -> Result<Report, Diagnostic> {
    let (program, names) = program_of(files)?;
    let target = only.and_then(|name| names.iter().position(|n| n == name));
    if only.is_some() && target.is_none() {
        return Err(Diagnostic::new(0, "the file isn't in its folder's program"));
    }
    Ok(report(&program, target, &names, solvers))
}

/// A folder's files parsed and checked as one program, with their names.
fn program_of(files: &[(String, String)]) -> Result<(syntax::Program, Vec<String>), Diagnostic> {
    let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    let mut program = syntax::Program {
        enums: Vec::new(),
        functions: Vec::new(),
        examples: Vec::new(),
    };
    for (index, (name, source)) in files.iter().enumerate() {
        let mut part = syntax::parse(source).map_err(|d| d.in_file(name))?;
        part.enums.iter_mut().for_each(|e| e.file = index);
        part.functions.iter_mut().for_each(|f| f.file = index);
        part.examples.iter_mut().for_each(|e| e.file = index);
        program.enums.extend(part.enums);
        program.functions.extend(part.functions);
        program.examples.extend(part.examples);
    }
    check::check(&mut program, &names)?;
    Ok((program, names))
}
