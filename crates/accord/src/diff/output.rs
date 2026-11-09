//! The comparison as text for people and as JSON for agents. The JSON schema
//! is documented in `docs/diff.md`; it is versioned and may change until the
//! first release.

use super::{Change, Comparison, Loosening, Verdict};
use crate::Inputs;

/// The comparison schema's version.
pub const VERSION: u32 = 1;

fn values(inputs: &Inputs) -> String {
    if inputs.is_empty() {
        return "every input".to_string();
    }
    inputs
        .iter()
        .map(|(n, v)| format!("{n} = {v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn loosening_text(loosening: &Loosening) -> String {
    match loosening {
        Loosening::Rejects(inputs) => {
            format!("the new contract no longer accepts {}", values(inputs))
        }
        Loosening::Allows(inputs, result) => format!(
            "for {}, the new contract allows result = {result}, which the old one ruled out",
            values(inputs)
        ),
    }
}

impl Verdict {
    /// The verdict's name, as the comparison writes it.
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Kept => "kept",
            Verdict::Weakened => "weakened",
            Verdict::Undecided => "undecided",
        }
    }
}

impl Change {
    /// The change's name, as the comparison writes it.
    pub fn name(&self) -> &'static str {
        match self {
            Change::Equivalent => "equivalent",
            Change::Stronger => "stronger",
            Change::Weaker(_) => "weaker",
            Change::Incomparable(_) => "incomparable",
            Change::Undecided(_) => "undecided",
            Change::Added => "added",
            Change::Removed => "removed",
            Change::Renamed(_) => "renamed",
            Change::SignatureChanged => "signature_changed",
            Change::NoContract => "no_contract",
        }
    }
}

/// The comparison as text: one line per function, then the verdict.
pub fn text(comparison: &Comparison) -> String {
    let mut lines: Vec<String> = comparison
        .functions
        .iter()
        .map(|f| {
            let detail = match &f.change {
                Change::Weaker(l) | Change::Incomparable(l) => format!(": {}", loosening_text(l)),
                Change::Undecided(reason) => format!(": {reason}"),
                Change::Renamed(old) => format!(": was `{old}`"),
                Change::Removed => ": its guarantees are gone".to_string(),
                Change::SignatureChanged => {
                    ": its parameter or result types changed, so the contracts can't be compared"
                        .to_string()
                }
                _ => String::new(),
            };
            format!("{:<12} {}{detail}", f.change.name(), f.name)
        })
        .collect();
    let weaker = comparison
        .functions
        .iter()
        .filter(|f| {
            matches!(
                f.change,
                Change::Weaker(_) | Change::Incomparable(_) | Change::Removed
            )
        })
        .count();
    lines.push(format!(
        "contracts    {} ({weaker} of {} functions loosened)",
        comparison.verdict.name(),
        comparison.functions.len()
    ));
    lines.join("\n")
}

fn inputs_json(inputs: &Inputs) -> serde_json::Value {
    inputs
        .iter()
        .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
        .collect()
}

/// The comparison as JSON, for the files at `old` and `new`.
pub fn json(comparison: &Comparison, old: &str, new: &str) -> String {
    let functions: Vec<serde_json::Value> = comparison
        .functions
        .iter()
        .map(|f| {
            let mut entry = serde_json::json!({ "name": f.name, "change": f.change.name() });
            match &f.change {
                Change::Weaker(l) | Change::Incomparable(l) => {
                    entry["loosening"] = match l {
                        Loosening::Rejects(inputs) => serde_json::json!({
                            "rejects": inputs_json(inputs),
                        }),
                        Loosening::Allows(inputs, result) => serde_json::json!({
                            "allows": { "inputs": inputs_json(inputs), "result": result },
                        }),
                    };
                }
                Change::Undecided(reason) => entry["reason"] = serde_json::json!(reason),
                Change::Renamed(old) => entry["was"] = serde_json::json!(old),
                _ => {}
            }
            entry
        })
        .collect();
    let document = serde_json::json!({
        "comparison": VERSION,
        "accord": env!("CARGO_PKG_VERSION"),
        "old": old,
        "new": new,
        "verdict": comparison.verdict.name(),
        "functions": functions,
    });
    serde_json::to_string_pretty(&document).unwrap_or_default()
}
