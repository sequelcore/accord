//! Stable condition identifiers. Numbers are never reassigned to new meanings.

/// A condition an agent can route without interpreting human wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// The source was rejected before proof obligations were generated.
    SourceRejected,
    /// A proof obligation was refuted.
    ObligationRefuted,
    /// A proof obligation did not settle, for any solver or replay reason.
    ObligationUndecided,
    /// An obligation cannot be checked by this compiler.
    ObligationUnsupported,
    /// A declared example failed.
    ExampleFailed,
    /// The contract admits two results outside declared freedoms.
    OpenCase,
    /// A behavior-changing mutant meets the contract.
    SurvivingMutant,
    /// Some mutants were undecided or untried.
    MutationIncomplete,
    /// The required solvers could not be located.
    SolverUnavailable,
    /// The open-case search did not settle.
    OpennessUndecided,
    /// The input could not be read.
    InputUnavailable,
    /// A function has no checked obligation or example.
    NoEvidence,
    /// An obligation wasn't proved from a callee's contract, which allows a
    /// result its body never gives.
    WeakCalleeContract,
    /// A function calls one that isn't verified, whose contract its proofs
    /// assume.
    UnverifiedCallee,
}

impl DiagnosticCode {
    /// The entire allocated catalog, including conditions with no current source producer.
    pub const ALL: [Self; 14] = [
        Self::SourceRejected,
        Self::ObligationRefuted,
        Self::ObligationUndecided,
        Self::ObligationUnsupported,
        Self::ExampleFailed,
        Self::OpenCase,
        Self::SurvivingMutant,
        Self::MutationIncomplete,
        Self::SolverUnavailable,
        Self::OpennessUndecided,
        Self::InputUnavailable,
        Self::NoEvidence,
        Self::WeakCalleeContract,
        Self::UnverifiedCallee,
    ];

    /// The permanent machine identifier, independent of severity and wording.
    pub fn name(self) -> &'static str {
        match self {
            Self::SourceRejected => "ACC0001",
            Self::ObligationRefuted => "ACC0002",
            Self::ObligationUndecided => "ACC0003",
            Self::ObligationUnsupported => "ACC0004",
            Self::ExampleFailed => "ACC0005",
            Self::OpenCase => "ACC0006",
            Self::SurvivingMutant => "ACC0007",
            Self::MutationIncomplete => "ACC0008",
            Self::SolverUnavailable => "ACC0009",
            Self::OpennessUndecided => "ACC0010",
            Self::InputUnavailable => "ACC0011",
            Self::NoEvidence => "ACC0012",
            Self::WeakCalleeContract => "ACC0013",
            Self::UnverifiedCallee => "ACC0014",
        }
    }

    /// A routing hint, never authority to approve or weaken a specification.
    pub fn action(self) -> &'static str {
        match self {
            Self::SourceRejected => "repair_source",
            Self::ObligationRefuted => "inspect_counterexample",
            Self::ObligationUndecided | Self::OpennessUndecided => "inspect_solver",
            Self::ObligationUnsupported => "report_unsupported",
            Self::ExampleFailed => "inspect_example",
            Self::OpenCase | Self::SurvivingMutant => "review_specification",
            Self::MutationIncomplete => "inspect_strength_limits",
            Self::SolverUnavailable | Self::InputUnavailable => "repair_environment",
            Self::NoEvidence => "add_checks",
            Self::WeakCalleeContract => "strengthen_callee_contract",
            Self::UnverifiedCallee => "verify_callee",
        }
    }
}
