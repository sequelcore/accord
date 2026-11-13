//! Structured projection of the same review record as terminal text.
use super::{VERSION, review};
use crate::{
    Definition, ExampleRun, Inputs, Openness, Outcome, Refutation, Report, Run, Strength,
    TraceStep, Type, Verdict,
};
use serde_json::{Value, json};

/// A pre-check rejection or unavailable prerequisite; no proof standing exists.
pub fn rejection(diagnostic: &crate::Diagnostic, file: &str) -> String {
    let place = diagnostic.file.as_ref().map_or_else(
        || file.to_string(),
        |name| std::path::Path::new(file).join(name).display().to_string(),
    );
    json!({
        "receipt": VERSION, "accord": env!("CARGO_PKG_VERSION"), "file": file,
        "status": if matches!(diagnostic.code, crate::DiagnosticCode::SolverUnavailable | crate::DiagnosticCode::InputUnavailable) { "unavailable" } else { "rejected" },
        "standing": null, "definitions": [],
        "diagnostics": [{"code": diagnostic.code.name(), "action": diagnostic.code.action(), "authority": "compiler", "severity": "error", "file": place, "line": if diagnostic.line == 0 { None } else { Some(diagnostic.line) }, "message": diagnostic.message, "evidence": null}],
    }).to_string()
}

fn type_json(ty: &Type) -> Value {
    match ty {
        Type::Bool => json!({"kind": "bool"}),
        Type::Int(width) => json!({"kind": "integer", "width": width.name()}),
        Type::Text => json!({"kind": "text"}),
        Type::Enum(name) => json!({"kind": "enum", "name": name}),
        Type::List(element) => json!({"kind": "list", "element": type_json(element)}),
    }
}

fn value_json(value: crate::interpret::Value) -> Value {
    use crate::interpret::Value as V;
    match value {
        V::Bool(b) => json!(b),
        V::Int(n) => json!(n.to_string()),
        V::Text(s) => json!(s),
        V::Variant(_, variant) => json!(variant),
        V::List(items) => items.into_iter().map(value_json).collect(),
    }
}

fn typed(source: &str, ty: &Type) -> Value {
    match crate::interpret::read(source, ty) {
        Some(value) => {
            json!({"type": type_json(ty), "value": value_json(value), "source": source, "valid": true})
        }
        None => json!({"type": type_json(ty), "value": null, "source": source, "valid": false}),
    }
}

fn inputs_json(inputs: &Inputs, params: &[(String, Type)]) -> Value {
    inputs
        .iter()
        .map(|(name, value)| {
            let mut out = match params.iter().find(|(n, _)| n == name) {
                Some((_, ty)) => typed(value, ty),
                None => json!({"source": value, "valid": false, "value": null}),
            };
            out["name"] = json!(name);
            out
        })
        .collect()
}

fn trace_json(steps: &[TraceStep], definition: &Definition) -> Value {
    steps
        .iter()
        .map(|step| {
            json!({
                "call": step.call,
                "state": step.state.as_ref().map(|state| inputs_json(state, &definition.params)),
            })
        })
        .collect()
}

fn verdict_json(verdict: &Verdict, definition: &Definition) -> Value {
    let existential = matches!(
        verdict.kind,
        crate::ObligationKind::RowReachability | crate::ObligationKind::Allowed
    );
    let mut out = json!({
        "kind": verdict.kind.name(),
        "polarity": if existential { "existential" } else { "universal" },
        "clause": verdict.clause,
        "line": verdict.line,
        "key": verdict.key,
        "reused": verdict.reused,
    });
    let params = if verdict.parameters.is_empty() {
        &definition.params
    } else {
        &verdict.parameters
    };
    let fields = match &verdict.outcome {
        Outcome::Proved { witness } => {
            let mut fields = json!({ "outcome": "proved" });
            if let Some(inputs) = witness {
                fields["witness"] = inputs_json(inputs, params);
            }
            fields
        }
        Outcome::Refuted {
            evidence: Refutation::Counterexample(inputs),
        } => json!({ "outcome": "refuted", "counterexample": inputs_json(inputs, params) }),
        Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        } => json!({ "outcome": "refuted", "no_such_input": true }),
        Outcome::Refuted {
            evidence: Refutation::Trace(steps),
        } => json!({ "outcome": "refuted", "trace": trace_json(steps, definition) }),
        Outcome::NotInductive { inputs } => json!({
            "outcome": "not_inductive",
            "counterexample": inputs_json(inputs, params),
        }),
        Outcome::Undecided { reason } => json!({ "outcome": "undecided", "reason": reason }),
        Outcome::Unsupported { reason } => json!({ "outcome": "unsupported", "reason": reason }),
        Outcome::WeakCallee { inputs, calls } => json!({
            "outcome": "weak_callee",
            "counterexample": inputs_json(inputs, params),
            "calls": calls
                .iter()
                .map(|(call, value)| json!({ "call": call, "value": value }))
                .collect::<Vec<_>>(),
        }),
    };
    if let (Some(out), Some(fields)) = (out.as_object_mut(), fields.as_object()) {
        out.extend(fields.clone());
    }
    if let Some(exploration) = &verdict.exploration {
        out["exploration"] = json!({
            "states": exploration.states,
            "complete": exploration.complete,
        });
        if let Some(trace) = &exploration.trace {
            out["exploration"]["trace"] = trace_json(trace, definition);
        }
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

/// `source` read as the definition's result type, which a function has.
fn result_typed(source: &str, definition: &Definition) -> Value {
    definition
        .result_type
        .as_ref()
        .map_or(Value::Null, |ty| typed(source, ty))
}

fn openness_json(openness: &Openness, definition: &Definition) -> Value {
    match openness {
        Openness::Determined => json!({ "determined": true }),
        Openness::Open {
            inputs,
            results,
            code,
        } => json!({
            "open": {
                "inputs": inputs_json(inputs, &definition.params),
                "results": [result_typed(&results.0, definition), result_typed(&results.1, definition)],
                "implementation": result_typed(code, definition),
            }
        }),
        Openness::Undecided { reason } => json!({ "undecided": reason }),
    }
}

fn strength_json(strength: &Strength, definition: &Definition) -> Value {
    json!({
        "tried": strength.tried,
        "caught": strength.caught,
        "equivalent": strength.equivalent,
        "undecided": strength.undecided,
        "not_tried": strength.not_tried,
        "survivors": strength
            .survivors
            .iter()
            .map(|s| json!({
                "change": s.change,
                "inputs": inputs_json(&s.inputs, &definition.params),
                "original": result_typed(&s.original, definition),
                "mutant": result_typed(&s.mutant, definition),
            }))
            .collect::<Vec<_>>(),
    })
}

/// The receipt as JSON, for the file at `file`.
pub fn json(report: &Report, file: &str) -> String {
    let definitions: Vec<Value> = report
        .definitions
        .iter()
        .map(|definition| definition_json(report, definition))
        .collect();
    let receipt = json!({
        "receipt": VERSION,
        "status": "checked",
        "diagnostics": super::diagnostics::findings(report),
        "accord": env!("CARGO_PKG_VERSION"),
        "file": file,
        "standing": report.standing.name(),
        "definitions": definitions,
    });
    serde_json::to_string_pretty(&receipt).unwrap_or_default()
}

pub(super) fn definition_json(report: &Report, definition: &Definition) -> Value {
    let mut value = json!({
        "signature": review::signature(definition),
        "line": definition.line,
        "parameters": definition.params.iter().map(|(name, ty)| json!({"name": name, "type": type_json(ty)})).collect::<Vec<_>>(),
        "result_type": definition.result_type.as_ref().map(type_json),
        "types": definition.enums.iter().map(|(name, variants)| json!({"name": name, "variants": variants})).collect::<Vec<_>>(),
        "support": definition.support.iter().map(|(line, text)| json!({"line": line, "text": text})).collect::<Vec<_>>(),
        "specification": definition.specification.iter().map(|(line, text)| json!({"line": line, "text": text})).collect::<Vec<_>>(),
        "not_proved": review::limits(report, definition).into_iter().map(|(kind, message)| json!({"kind": kind, "message": message})).collect::<Vec<_>>(),
        "name": definition.name,
        "intent": definition.intent,
        "file": definition.file,
        "kind": definition.kind.name(),
        "hash": definition.hash,
        "standing": definition.standing.name(),
        "unverified_callees": definition.unverified_callees,
        "obligations": report
            .verdicts
            .iter()
            .filter(|v| v.definition == definition.hash && v.function == definition.name)
            .map(|v| verdict_json(v, definition))
            .collect::<Vec<_>>(),
        "examples": report
            .examples
            .iter()
            .filter(|e| e.function == definition.name)
            .map(example_json)
            .collect::<Vec<_>>(),
        "strength": report
            .strength
            .iter()
            .find(|s| s.function == definition.name)
            .map(|s| strength_json(s, definition)),
        "openness": report
            .openness
            .iter()
            .find(|d| d.function == definition.name)
            .map(|d| {
                let mut value = openness_json(&d.openness, definition);
                value["free"] = json!(d.free);
                value
            }),
    });
    let mut content = value.clone();
    if let Some(obligations) = content["obligations"].as_array_mut() {
        for obligation in obligations {
            if let Some(o) = obligation.as_object_mut() {
                o.remove("reused");
            }
        }
    }
    value["review_hash"] = json!(crate::identity::hash(&content.to_string()));
    value
}
