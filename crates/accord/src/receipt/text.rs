//! The receipt as text: each function's review view, then the program's
//! standing and its diagnostics.

use super::{diagnostics, json, review};
use crate::{
    Determinacy, ExampleRun, Inputs, Openness, Outcome, Refutation, Report, Run, Strength,
    TraceStep, Verdict,
};

fn values(inputs: &Inputs) -> String {
    if inputs.is_empty() {
        return "no inputs".to_string();
    }
    inputs
        .iter()
        .map(|(n, v)| format!("{n} = {v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A run, one step to a line.
fn trace_text(steps: &[TraceStep]) -> String {
    steps
        .iter()
        .map(|step| match &step.state {
            Some(state) => format!("             {} leaves {}", step.call, values(state)),
            None => format!("             {} fails here", step.call),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What exploring the reachable states covered, for an obligation that
/// consulted it.
fn exploration_text(verdict: &Verdict) -> Option<String> {
    let exploration = verdict.exploration.as_ref()?;
    let mut line = format!(
        "             explored {} states, {} (trusted, not proved)",
        exploration.states,
        if exploration.complete {
            "all of them"
        } else {
            "not all"
        }
    );
    if let Some(trace) = &exploration.trace {
        line = format!("{line}\n{}", trace_text(trace));
    }
    Some(line)
}

fn verdict_text(verdict: &Verdict) -> String {
    match exploration_text(verdict) {
        Some(note) => format!("{}\n{note}", outcome_text(verdict)),
        None => outcome_text(verdict),
    }
}

fn outcome_text(verdict: &Verdict) -> String {
    let reused = if verdict.reused { "  [reused]" } else { "" };
    let clause = if verdict.kind == crate::ObligationKind::RowReachability {
        "some admitted input reaches this row first (existential)"
    } else {
        &verdict.clause
    };
    let place = format!(
        "{} (line {}): {}{reused}",
        verdict.function, verdict.line, clause
    );
    match &verdict.outcome {
        Outcome::Proved { witness: None } => format!("proved       {place}"),
        Outcome::Proved {
            witness: Some(inputs),
        } => format!(
            "proved       {place}\n             reached with {}",
            values(inputs)
        ),
        Outcome::Refuted {
            evidence: Refutation::Counterexample(inputs),
        } => format!(
            "refuted      {place}\n             fails with {}",
            values(inputs)
        ),
        Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        } => format!("refuted      {place}\n             no input reaches it"),
        Outcome::Refuted {
            evidence: Refutation::Trace(steps),
        } => format!("refuted      {place}\n{}", trace_text(steps)),
        Outcome::NotInductive { inputs } => format!(
            "not inductive {place}\n             not proved from the invariants alone: from {}, which no run from an init was found to reach; a helper invariant may settle it",
            values(inputs)
        ),
        Outcome::Undecided { reason } => format!("undecided    {place}\n             {reason}"),
        Outcome::Unsupported { reason } => {
            format!("unsupported  {place}\n             {reason}")
        }
        Outcome::WeakCallee { inputs, calls } => format!(
            "weak callee  {place}\n             not proved from the contracts it calls: fails with {} only when {}, which the callee's contract allows and its body doesn't give",
            values(inputs),
            calls
                .iter()
                .map(|(call, value)| format!("`{call}` gives {value}"))
                .collect::<Vec<_>>()
                .join(" and ")
        ),
    }
}

fn example_text(example: &ExampleRun) -> String {
    let place = format!(
        "{} (line {}): {}",
        example.function, example.line, example.text
    );
    match &example.run {
        Run::Passed => format!("passed       {place}"),
        Run::Failed { reason } => format!("failed       {place}\n             {reason}"),
    }
}

fn strength_text(strength: &Strength) -> Vec<String> {
    let mut lines: Vec<String> = strength
        .survivors
        .iter()
        .map(|s| {
            format!(
                "weak         {}: {}
             for {}, the code gives {} and the change gives {}; the contract allows both",
                strength.function,
                s.change,
                values(&s.inputs),
                s.original,
                s.mutant
            )
        })
        .collect();
    let not_tried = if strength.not_tried > 0 {
        format!(", {} not tried", strength.not_tried)
    } else {
        String::new()
    };
    lines.push(format!(
        "strength     {}: {} {}: {} caught, {} equivalent, {} surviving, {} undecided{not_tried}",
        strength.function,
        strength.tried,
        if strength.tried == 1 {
            "mutant"
        } else {
            "mutants"
        },
        strength.caught,
        strength.equivalent,
        strength.survivors.len(),
        strength.undecided
    ));
    lines
}

fn openness_text(d: &Determinacy) -> Vec<String> {
    let mut lines: Vec<String> = d
        .free
        .iter()
        .map(|clause| format!("free         {}: {clause}", d.function))
        .collect();
    lines.extend(match &d.openness {
        Openness::Determined => None,
        Openness::Open {
            inputs,
            results,
            code,
        } => Some(format!(
            "open         {}: for {}, the contract allows both {} and {} (the code gives {})",
            d.function,
            values(inputs),
            results.0,
            results.1,
            code
        )),
        Openness::Undecided { reason } => Some(format!(
            "undecided    {}: whether the contract determines the result\n             {reason}",
            d.function
        )),
    });
    lines
}

/// The receipt as text: each obligation and example, each open case, each
/// contract's strength when it was judged, then each function's standing and
/// the file's.
pub fn text(report: &Report) -> String {
    let mut lines = Vec::new();
    for definition in &report.definitions {
        let file = definition
            .file
            .as_ref()
            .map_or(String::new(), |f| format!(" ({f})"));
        lines.push(format!(
            "{:<13}{}{file}",
            definition.kind.name(),
            review::signature(definition)
        ));
        let metadata = json::definition_json(report, definition);
        lines.push(format!(
            "review_hash  {}",
            metadata["review_hash"].as_str().unwrap_or_default()
        ));
        for (name, variants) in &definition.enums {
            lines.push(format!(
                "type         enum {name} {{ {} }}",
                variants.join(", ")
            ));
        }
        lines.push(match &definition.intent {
            Some(intent) => format!(
                "intent       {}{file} (unchecked): {intent}",
                definition.name
            ),
            None => format!("intent       {}{file}: not declared", definition.name),
        });
        if definition.specification.is_empty() {
            lines.push("specification none declared".to_string());
        }
        for (line, clause) in &definition.specification {
            lines.push(format!("specification line {line}: {clause}"));
        }
        let verdicts: Vec<_> = report
            .verdicts
            .iter()
            .filter(|v| v.function == definition.name)
            .collect();
        lines.extend(verdicts.iter().map(|v| verdict_text(v)));
        if !verdicts
            .iter()
            .any(|v| matches!(v.outcome, Outcome::Refuted { .. }))
        {
            lines.push("counterexamples none reported (this alone is not a proof)".to_string());
        }
        let examples: Vec<_> = report
            .examples
            .iter()
            .filter(|e| e.function == definition.name)
            .collect();
        if examples.is_empty() {
            lines.push("examples     none declared".to_string());
        }
        lines.extend(examples.into_iter().map(example_text));
        match report
            .openness
            .iter()
            .find(|o| o.function == definition.name)
        {
            Some(o) => {
                lines.extend(openness_text(o));
                if matches!(o.openness, Openness::Determined) {
                    lines.push(
                        "open_cases   none outside declared freedoms (search settled)".to_string(),
                    );
                }
                if o.free.is_empty() {
                    lines.push("freedoms     none declared".to_string());
                }
            }
            None => lines.push("open_cases   not searched (no ensures)".to_string()),
        }
        match report
            .strength
            .iter()
            .find(|s| s.function == definition.name)
        {
            Some(s) => lines.extend(strength_text(s)),
            None => lines.push("strength     not run".to_string()),
        }
        for (kind, message) in review::limits(report, definition) {
            lines.push(format!("not_proved   {kind}: {message}"));
        }
        let callees = if definition.unverified_callees.is_empty() {
            String::new()
        } else {
            format!(
                " (calls {}, not verified)",
                definition
                    .unverified_callees
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        lines.push(format!(
            "standing     {}{file}: {}{callees}",
            definition.name,
            definition.standing.name()
        ));
    }
    for finding in diagnostics::findings(report) {
        lines.push(format!(
            "diagnostic   {} {}: {} ({})",
            finding["code"].as_str().unwrap_or_default(),
            finding["function"].as_str().unwrap_or_default(),
            finding["action"].as_str().unwrap_or_default(),
            finding["evidence"].as_str().unwrap_or_default()
        ));
    }
    let proved = report
        .verdicts
        .iter()
        .filter(|v| matches!(v.outcome, Outcome::Proved { .. }))
        .count();
    let passed = report
        .examples
        .iter()
        .filter(|e| e.run == Run::Passed)
        .count();
    let reused = report.verdicts.iter().filter(|v| v.reused).count();
    let what = if report.definitions.iter().any(|d| d.file.is_some()) {
        "the folder"
    } else {
        "the file"
    };
    lines.push(format!(
        "standing     {what}: {} ({proved} of {} obligations proved, {reused} reused from the ledger; {passed} of {} examples passed)",
        report.standing.name(),
        report.verdicts.len(),
        report.examples.len()
    ));
    lines.join("\n")
}
