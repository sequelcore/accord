//! Open cases: an admitted input for which a function's contract allows two
//! different results. Any implementation that picks either one proves, so
//! the proof doesn't pin down what the function does there. Only functions
//! with an `ensures` are searched; whether a freedom is intended is a
//! person's decision, so an open case makes a function incomplete, not
//! failed.

use crate::identity::Unit;
use crate::interpret;
use crate::obligation::{self, Polarity};
use crate::syntax::ClauseKind;
use crate::{Inputs, Outcome, Refutation, Solvers};

/// Whether a function's contract determines its result.
#[derive(Clone, Debug, PartialEq)]
pub enum Openness {
    /// One result for every admitted input.
    Determined,
    /// Two results the contract both allows for one input.
    Open {
        /// The input.
        inputs: Inputs,
        /// The two results.
        results: (String, String),
        /// What the code gives for that input.
        code: String,
    },
    /// The solvers didn't settle it.
    Undecided {
        /// Why.
        reason: String,
    },
}

/// One function's openness.
#[derive(Clone, Debug, PartialEq)]
pub struct Determinacy {
    /// The function.
    pub function: String,
    /// Whether its contract determines its result.
    pub openness: Openness,
}

fn search(unit: &Unit, solvers: &Solvers) -> Option<Determinacy> {
    let function = unit.program.functions.first()?;
    if !function
        .clauses
        .iter()
        .any(|c| c.kind == ClauseKind::Ensures)
    {
        return None;
    }
    let script = obligation::open_script(function, &obligation::datatypes(&unit.program));
    let mut named: Vec<String> = function.params.iter().map(|(n, _)| n.clone()).collect();
    named.extend(["r0".to_string(), "r1".to_string()]);
    let read = |inputs: &Inputs, name: &str| {
        let raw = inputs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .ok_or(format!("no value for `{name}`"))?;
        interpret::read(raw, &function.ret).ok_or(format!("`{raw}` isn't a result"))
    };
    // Both results must meet the contract and differ.
    let replay = |inputs: &Inputs| -> Result<(), String> {
        let names = interpret::admitted(function, inputs)?;
        let (first, second) = (read(inputs, "r0")?, read(inputs, "r1")?);
        let both = interpret::meets(function, &names, &first)?
            && interpret::meets(function, &names, &second)?;
        if both && first != second {
            Ok(())
        } else {
            Err("the contract doesn't allow both there".to_string())
        }
    };
    let openness = match solvers
        .decide(&script, &named, Polarity::Existential, replay)
        .0
    {
        Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        } => Openness::Determined,
        Outcome::Proved {
            witness: Some(witness),
        } => {
            let inputs: Inputs = witness
                .iter()
                .filter(|(n, _)| n != "r0" && n != "r1")
                .cloned()
                .collect();
            let found = (|| {
                let names = interpret::admitted(function, &inputs)?;
                let code = interpret::result(function, &names)?;
                Ok::<_, String>((read(&witness, "r0")?, read(&witness, "r1")?, code))
            })();
            match found {
                Ok((first, second, code)) => Openness::Open {
                    inputs: unit.source_inputs(&inputs),
                    results: (
                        unit.source_value(&first.source()),
                        unit.source_value(&second.source()),
                    ),
                    code: unit.source_value(&code.source()),
                },
                Err(reason) => Openness::Undecided { reason },
            }
        }
        Outcome::Undecided { reason } | Outcome::Unsupported { reason } => {
            Openness::Undecided { reason }
        }
        Outcome::Proved { witness: None } | Outcome::Refuted { .. } => Openness::Undecided {
            reason: "the search gave no witness".to_string(),
        },
    };
    Some(Determinacy {
        function: unit.source.name.clone(),
        openness,
    })
}

/// The openness of every function with an `ensures`, in source order.
pub(crate) fn determinacy(units: &[Unit], solvers: &Solvers) -> Vec<Determinacy> {
    units
        .iter()
        .filter_map(|unit| search(unit, solvers))
        .collect()
}
