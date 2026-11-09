//! Review metadata and limits shared by the text and JSON projections.
use crate::{Definition, Openness, Outcome, Report, Run};

pub(super) fn signature(d: &Definition) -> String {
    let params = d
        .params
        .iter()
        .map(|(name, ty)| format!("{name}: {}", ty.name()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("fn {}({params}) -> {}", d.name, d.result_type.name())
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
