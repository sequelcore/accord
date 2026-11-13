//! The evidence ledger: a local cache of what the solvers established, so an
//! obligation that hasn't changed isn't decided again.
//!
//! An entry's key is the hash of the obligation's script together with the
//! exact tools that decided it: Z3's and cvc5's versions and the hash of the
//! running program, so a rebuilt compiler or another solver re-checks
//! everything. Only `proved` and `refuted` are stored; an undecided obligation
//! is evidence of nothing and is decided again on every run. Each entry is a
//! file of its own, written whole and then renamed into place, so an entry
//! that can't be read is a miss: it is decided again, never trusted.
//!
//! Outcomes are stored as the normalized function's, so a rename still finds
//! them and they read back in the new names.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use crate::identity::{hash, hash_bytes};
use crate::{Inputs, Outcome, Refutation};

#[derive(Clone, Debug)]
pub(crate) struct Ledger {
    dir: PathBuf,
    /// The tools that decide obligations, as part of every key.
    tools: String,
}

fn first_line(program: &Path) -> Option<String> {
    let output = Command::new(program).arg("--version").output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().next()?.trim().to_string();
    (output.status.success() && !line.is_empty()).then_some(line)
}

impl Ledger {
    /// A ledger in `dir`, or `None` when the tools can't be identified: then
    /// nothing is reused.
    pub fn open(dir: &Path, z3: &Path, cvc5: &Path) -> Option<Ledger> {
        let program = std::fs::read(std::env::current_exe().ok()?).ok()?;
        let tools = format!(
            "accord {} {}\n{}\n{}\n",
            env!("CARGO_PKG_VERSION"),
            hash_bytes(&program),
            first_line(z3)?,
            first_line(cvc5)?
        );
        Some(Ledger {
            dir: dir.join("evidence"),
            tools,
        })
    }

    fn path(&self, script: &str) -> PathBuf {
        self.dir
            .join(format!("{}.json", hash(&format!("{}{script}", self.tools))))
    }

    /// The stored outcome of `script`, if there is one and it can be read.
    pub fn get(&self, script: &str) -> Option<Outcome> {
        let text = std::fs::read_to_string(self.path(script)).ok()?;
        from_json(&serde_json::from_str(&text).ok()?)
    }

    /// Stores `outcome` for `script` if it's evidence. A failure to write is
    /// ignored: the obligation is simply decided again next time.
    pub fn put(&self, script: &str, outcome: &Outcome) {
        let Some(value) = to_json(outcome) else {
            return;
        };
        let path = self.path(script);
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let written = std::fs::create_dir_all(&self.dir)
            .and_then(|()| std::fs::write(&temporary, value.to_string()))
            .and_then(|()| std::fs::rename(&temporary, &path));
        if written.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
    }
}

fn inputs_to_json(inputs: &Inputs) -> Value {
    inputs.iter().map(|(n, v)| json!([n, v])).collect()
}

fn inputs_from_json(value: &Value) -> Option<Inputs> {
    value
        .as_array()?
        .iter()
        .map(|pair| {
            let name = pair.get(0)?.as_str()?.to_string();
            let value = pair.get(1)?.as_str()?.to_string();
            Some((name, value))
        })
        .collect()
}

fn to_json(outcome: &Outcome) -> Option<Value> {
    Some(match outcome {
        Outcome::Proved { witness: None } => json!({ "proved": null }),
        Outcome::Proved {
            witness: Some(inputs),
        } => json!({ "proved": inputs_to_json(inputs) }),
        Outcome::Refuted {
            evidence: Refutation::Counterexample(inputs),
        } => json!({ "refuted": inputs_to_json(inputs) }),
        Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        } => json!({ "refuted": null }),
        Outcome::WeakCallee { inputs, .. } => json!({ "weak_callee": inputs_to_json(inputs) }),
        Outcome::Undecided { .. }
        | Outcome::Unsupported { .. }
        | Outcome::NotInductive { .. }
        | Outcome::Refuted {
            evidence: Refutation::Trace(_),
        } => return None,
    })
}

fn from_json(value: &Value) -> Option<Outcome> {
    if let Some(witness) = value.get("proved") {
        return Some(Outcome::Proved {
            witness: if witness.is_null() {
                None
            } else {
                Some(inputs_from_json(witness)?)
            },
        });
    }
    if let Some(inputs) = value.get("weak_callee") {
        return Some(Outcome::WeakCallee {
            inputs: inputs_from_json(inputs)?,
            calls: Vec::new(),
        });
    }
    let refuted = value.get("refuted")?;
    Some(Outcome::Refuted {
        evidence: if refuted.is_null() {
            Refutation::NoSuchInput
        } else {
            Refutation::Counterexample(inputs_from_json(refuted)?)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{from_json, to_json};
    use crate::{Outcome, Refutation};

    #[test]
    fn evidence_round_trips_and_nothing_else_is_stored() {
        let inputs = vec![("p0".to_string(), "-3".to_string())];
        for outcome in [
            Outcome::Proved { witness: None },
            Outcome::Proved {
                witness: Some(inputs.clone()),
            },
            Outcome::Refuted {
                evidence: Refutation::Counterexample(inputs.clone()),
            },
            Outcome::Refuted {
                evidence: Refutation::NoSuchInput,
            },
            Outcome::WeakCallee {
                inputs: inputs.clone(),
                calls: Vec::new(),
            },
        ] {
            let stored = to_json(&outcome).expect("evidence is stored");
            assert_eq!(from_json(&stored), Some(outcome));
        }
        let undecided = Outcome::Undecided {
            reason: "timeout".to_string(),
        };
        assert_eq!(to_json(&undecided), None);
        assert_eq!(from_json(&serde_json::json!({ "other": 1 })), None);
        assert_eq!(from_json(&serde_json::json!({ "refuted": [["p0"]] })), None);
    }
}
