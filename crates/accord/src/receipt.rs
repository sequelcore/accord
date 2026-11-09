//! The receipt: one check's report, as text for people and as JSON for
//! agents. Both carry the same record. The JSON schema is documented in
//! `docs/receipt.md`; it is versioned and may change until the first release.

use serde_json::{Value, json};

use crate::{ExampleRun, Inputs, Outcome, Refutation, Report, Run, Standing, Verdict};

/// The receipt schema's version.
pub const VERSION: u32 = 1;

impl Standing {
    /// The standing's name, as the receipt writes it.
    pub fn name(self) -> &'static str {
        match self {
            Standing::Verified => "verified",
            Standing::Incomplete => "incomplete",
            Standing::Failed => "failed",
            Standing::Unverified => "unverified",
        }
    }
}

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

fn verdict_text(verdict: &Verdict) -> String {
    let reused = if verdict.reused { "  [reused]" } else { "" };
    let place = format!(
        "{} (line {}): {}{reused}",
        verdict.function, verdict.line, verdict.clause
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
        Outcome::Undecided { reason } => format!("undecided    {place}\n             {reason}"),
        Outcome::Unsupported { reason } => {
            format!("unsupported  {place}\n             {reason}")
        }
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

/// The receipt as text: each obligation and example, then each function's
/// standing and the file's.
pub fn text(report: &Report) -> String {
    let mut lines: Vec<String> = report.verdicts.iter().map(verdict_text).collect();
    lines.extend(report.examples.iter().map(example_text));
    for definition in &report.definitions {
        lines.push(format!(
            "standing     {}: {}",
            definition.name,
            definition.standing.name()
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
    lines.push(format!(
        "standing     the file: {} ({proved} of {} obligations proved, {reused} reused from the ledger; {passed} of {} examples passed)",
        report.standing.name(),
        report.verdicts.len(),
        report.examples.len()
    ));
    lines.join("\n")
}

fn inputs_json(inputs: &Inputs) -> Value {
    inputs
        .iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect()
}

fn verdict_json(verdict: &Verdict) -> Value {
    let mut out = json!({
        "clause": verdict.clause,
        "line": verdict.line,
        "key": verdict.key,
        "reused": verdict.reused,
    });
    let fields = match &verdict.outcome {
        Outcome::Proved { witness } => {
            let mut fields = json!({ "outcome": "proved" });
            if let Some(inputs) = witness {
                fields["witness"] = inputs_json(inputs);
            }
            fields
        }
        Outcome::Refuted {
            evidence: Refutation::Counterexample(inputs),
        } => json!({ "outcome": "refuted", "counterexample": inputs_json(inputs) }),
        Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        } => json!({ "outcome": "refuted", "no_such_input": true }),
        Outcome::Undecided { reason } => json!({ "outcome": "undecided", "reason": reason }),
        Outcome::Unsupported { reason } => json!({ "outcome": "unsupported", "reason": reason }),
    };
    if let (Some(out), Some(fields)) = (out.as_object_mut(), fields.as_object()) {
        out.extend(fields.clone());
    }
    out
}

fn example_json(example: &ExampleRun) -> Value {
    match &example.run {
        Run::Passed => json!({ "text": example.text, "line": example.line, "run": "passed" }),
        Run::Failed { reason } => json!({
            "text": example.text,
            "line": example.line,
            "run": "failed",
            "reason": reason,
        }),
    }
}

/// The receipt as JSON, for the file at `file`.
pub fn json(report: &Report, file: &str) -> String {
    let definitions: Vec<Value> = report
        .definitions
        .iter()
        .map(|definition| {
            json!({
                "name": definition.name,
                "kind": "function",
                "hash": definition.hash,
                "standing": definition.standing.name(),
                "obligations": report
                    .verdicts
                    .iter()
                    .filter(|v| v.definition == definition.hash && v.function == definition.name)
                    .map(verdict_json)
                    .collect::<Vec<_>>(),
                "examples": report
                    .examples
                    .iter()
                    .filter(|e| e.function == definition.name)
                    .map(example_json)
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    let receipt = json!({
        "receipt": VERSION,
        "accord": env!("CARGO_PKG_VERSION"),
        "file": file,
        "standing": report.standing.name(),
        "definitions": definitions,
    });
    serde_json::to_string_pretty(&receipt).unwrap_or_default()
}
