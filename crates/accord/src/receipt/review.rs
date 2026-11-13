//! Review metadata and limits shared by the text and JSON projections.
use crate::{Definition, Openness, Outcome, Report, Run};

pub(super) fn signature(d: &Definition) -> String {
    let Some(result_type) = &d.result_type else {
        return format!("machine {}", d.name);
    };
    let params = d
        .params
        .iter()
        .map(|(name, ty)| format!("{name}: {}", ty.name()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("fn {}({params}) -> {}", d.name, result_type.name())
}

pub(super) fn limits(report: &Report, d: &Definition) -> Vec<(&'static str, &'static str)> {
    let mut out = vec![
        (
            "intent_alignment",
            "The compiler does not prove that the specification captures the intended requirement.",
        ),
        (
            "integration",
            "Callers, external interpretation of inputs, and surrounding system behavior were not checked.",
        ),
        (
            "trusted_components",
            "The parser, obligation generator, solvers, and interpreter are trusted, not proved correct here.",
        ),
    ];
    if d.params
        .iter()
        .any(|(_, ty)| matches!(ty, crate::Type::List(_)))
    {
        out.push((
            "list_assumptions",
            "Every list is assumed to hold at most 4,294,967,295 elements, as a TypeScript array does, and `count` is known to the solvers only through trusted axioms: its bounds, when it is zero and when it is the length.",
        ));
    }
    let verdicts: Vec<_> = report
        .verdicts
        .iter()
        .filter(|v| v.function == d.name)
        .collect();
    if verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::Refuted { .. }))
    {
        out.push(("refuted_obligation", "At least one obligation was refuted; inspect its counterexample or unreachable-row finding."));
    }
    if verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::Undecided { .. }))
    {
        out.push((
            "undecided_obligation",
            "At least one obligation did not settle; no proof is claimed for it.",
        ));
    }
    if verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::WeakCallee { .. }))
    {
        out.push((
            "weak_callee_contract",
            "At least one obligation was not proved because a callee's contract allows results its body never gives.",
        ));
    }
    if !d.unverified_callees.is_empty() {
        out.push((
            "unverified_callee",
            "It calls a function that isn't verified; its own proofs assume that function's contract.",
        ));
    }
    if verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::NotInductive { .. }))
    {
        out.push((
            "not_inductive_obligation",
            "At least one obligation was not proved because a state that meets every invariant and helper invariant leaves them, though no run from an init was found to reach it; a helper invariant may settle it.",
        ));
    }
    if verdicts.iter().any(|v| v.exploration.is_some()) {
        out.push((
            "exploration",
            "Exploring a machine's reachable states runs the interpreter over finitely many states and arguments; what it covers is trusted, and it proves nothing.",
        ));
    }
    if verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::Unsupported { .. }))
    {
        out.push((
            "unsupported_obligation",
            "At least one obligation is outside the supported checks.",
        ));
    }
    let examples: Vec<_> = report
        .examples
        .iter()
        .filter(|e| e.function == d.name)
        .collect();
    if examples.is_empty() {
        out.push((
            "no_examples",
            "No declared example was executed for this function.",
        ));
    } else if examples.iter().any(|e| matches!(e.run, Run::Failed { .. })) {
        out.push(("failed_example", "At least one declared example failed."));
    }
    if d.specification
        .iter()
        .any(|(_, c)| c.starts_with("requires "))
    {
        out.push(("admitted_domain", "Guarantees apply only to inputs meeting requires; no behavior is promised for excluded inputs."));
    }
    if d.result_type.is_none() {
        return out;
    }
    match report.openness.iter().find(|o| o.function == d.name) {
        None => out.push(("no_postcondition", "No ensures was declared; no postcondition or open-case search establishes the intended result.")),
        Some(o) => {
            match o.openness {
                Openness::Open { .. } => out.push(("open_case", "The contract permits two results for an input outside declared freedoms.")),
                Openness::Undecided { .. } => out.push(("openness_undecided", "The open-case search did not settle.")),
                Openness::Determined => {}
            }
            if !o.free.is_empty() {
                out.push(("declared_freedoms", "Declared freedoms permit multiple contract-compliant results; their acceptability is not proved or approved here."));
            }
        }
    }
    match report.strength.iter().find(|s| s.function == d.name) {
        None => out.push((
            "mutation_not_run",
            "Mutation analysis was not run for this function.",
        )),
        Some(s) => {
            out.push(("mutation_sample", "Only the reported finite sample of mutants was checked; contract adequacy is not proved."));
            if !s.survivors.is_empty() {
                out.push(("surviving_mutants", "The contract allows behavior-changing mutants; inspect their distinguishing inputs."));
            }
            if s.undecided > 0 || s.not_tried > 0 {
                out.push((
                    "mutation_incomplete",
                    "Some mutants were undecided or not tried; they are not counted as caught.",
                ));
            }
        }
    }
    out
}
