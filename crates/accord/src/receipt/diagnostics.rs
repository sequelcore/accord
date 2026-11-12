//! Findings reference the canonical evidence in the receipt by JSON pointer.
use crate::{DiagnosticCode as C, Openness, Outcome, Report, Run, Standing};
use serde_json::{Value, json};

pub(super) fn findings(report: &Report) -> Vec<Value> {
    let mut out = Vec::new();
    if report.definitions.is_empty() {
        out.push(json!({"code": C::NoEvidence.name(), "action": C::NoEvidence.action(), "authority": "compiler", "severity": "error", "function": null, "definition": null, "file": null, "line": null, "evidence": "/definitions"}));
    }
    for (index, d) in report.definitions.iter().enumerate() {
        let mut add = |code: C, line: usize, evidence: String| {
            out.push(json!({
                "code": code.name(), "action": code.action(), "authority": "compiler",
                "severity": if matches!(code, C::SurvivingMutant | C::MutationIncomplete) { "warning" } else { "error" },
                "function": d.name, "definition": d.hash, "file": d.file,
                "line": line, "evidence": evidence,
            }));
        };
        for (i, v) in report
            .verdicts
            .iter()
            .filter(|v| v.function == d.name)
            .enumerate()
        {
            let code = match v.outcome {
                Outcome::Proved { .. } => None,
                Outcome::Refuted { .. } => Some(C::ObligationRefuted),
                Outcome::Undecided { .. } => Some(C::ObligationUndecided),
                Outcome::Unsupported { .. } => Some(C::ObligationUnsupported),
                Outcome::WeakCallee { .. } => Some(C::WeakCalleeContract),
            };
            if let Some(code) = code {
                add(
                    code,
                    v.line,
                    format!("/definitions/{index}/obligations/{i}"),
                );
            }
        }
        for (i, e) in report
            .examples
            .iter()
            .filter(|e| e.function == d.name)
            .enumerate()
        {
            if matches!(e.run, Run::Failed { .. }) {
                add(
                    C::ExampleFailed,
                    e.line,
                    format!("/definitions/{index}/examples/{i}"),
                );
            }
        }
        if let Some(o) = report.openness.iter().find(|o| o.function == d.name) {
            let code = match o.openness {
                Openness::Open { .. } => Some(C::OpenCase),
                Openness::Undecided { .. } => Some(C::OpennessUndecided),
                Openness::Determined => None,
            };
            if let Some(code) = code {
                add(code, d.line, format!("/definitions/{index}/openness"));
            }
        }
        if let Some(s) = report.strength.iter().find(|s| s.function == d.name) {
            for i in 0..s.survivors.len() {
                add(
                    C::SurvivingMutant,
                    d.line,
                    format!("/definitions/{index}/strength/survivors/{i}"),
                );
            }
            if s.undecided > 0 || s.not_tried > 0 {
                add(
                    C::MutationIncomplete,
                    d.line,
                    format!("/definitions/{index}/strength"),
                );
            }
        }
        if !d.unverified_callees.is_empty() {
            add(
                C::UnverifiedCallee,
                d.line,
                format!("/definitions/{index}/unverified_callees"),
            );
        }
        if d.standing == Standing::Unverified {
            add(C::NoEvidence, d.line, format!("/definitions/{index}"));
        }
    }
    out
}
