//! Contract strength: small, plausible mistakes planted in a function's body
//! (mutants), each judged against the function's own contract.
//!
//! A mutant the contract rules out is **caught**. One it can't tell apart is
//! either **equivalent**, proved to give the original's result for every
//! admitted input, and dropped; or it **survives**: the solvers give an input
//! on which the code and the mutant differ, and the contract allows both. A
//! surviving mutant makes the contract weak, and it is reported with that
//! input, since a bare list of survivors isn't something a person can act on.
//! Only functions with an `ensures` are mutated: without a contract there is
//! nothing to catch a mistake.

mod mutants;

use mutants::{describe, mutate};

use crate::identity::Unit;
use crate::interpret;
use crate::obligation::{self, Polarity};
use crate::syntax::ClauseKind;
use crate::{Inputs, Outcome, Refutation, Solvers};

/// The most mutants tried per function; the rest are counted as not tried.
pub(crate) const CAP: usize = 64;

/// A surviving mutant: the change, an input on which it differs from the
/// code, and the two results, which the contract both allows.
#[derive(Clone, Debug, PartialEq)]
pub struct Survivor {
    /// The change, in source terms.
    pub change: String,
    /// The input on which the code and the mutant differ.
    pub inputs: Inputs,
    /// The code's result there.
    pub original: String,
    /// The mutant's result there.
    pub mutant: String,
}

/// How one function's contract fared against its mutants.
#[derive(Clone, Debug, PartialEq)]
pub struct Strength {
    /// The function.
    pub function: String,
    /// Mutants tried.
    pub tried: usize,
    /// Mutants the contract ruled out.
    pub caught: usize,
    /// Mutants proved to behave like the code: not mistakes, so dropped.
    pub equivalent: usize,
    /// Mutants the solvers couldn't settle.
    pub undecided: usize,
    /// Mutants beyond the cap, not tried.
    pub not_tried: usize,
    /// Mutants the contract allows although they behave differently.
    pub survivors: Vec<Survivor>,
}

/// Runs every mutant of one normalized function, up to the cap.
fn judge(unit: &Unit, solvers: &Solvers) -> Option<Strength> {
    let original = unit.program.functions.first()?;
    if !original
        .clauses
        .iter()
        .any(|c| c.kind == ClauseKind::Ensures)
    {
        return None;
    }
    let datatypes = obligation::datatypes(&unit.program);
    let mut strength = Strength {
        function: unit.source.name.clone(),
        tried: 0,
        caught: 0,
        equivalent: 0,
        undecided: 0,
        not_tried: 0,
        survivors: Vec::new(),
    };
    let params: Vec<String> = original.params.iter().map(|(n, _)| n.clone()).collect();
    let mut named = params.clone();
    named.extend(["r0".to_string(), "r1".to_string()]);
    for n in 0.. {
        let mut mutant = original.clone();
        let Some(change) = mutate(&mut mutant, &unit.program, n) else {
            break;
        };
        if strength.tried == CAP {
            strength.not_tried += 1;
            continue;
        }
        strength.tried += 1;
        let (survives, differs) =
            obligation::mutant_scripts(original, &mutant, &unit.program, &datatypes);
        // A counterexample here must be an input on which the mutant breaks
        // the contract.
        let breaks = |inputs: &Inputs| -> Result<(), String> {
            let names = interpret::admitted(original, inputs)?;
            let result = interpret::result(&mutant, &names, &unit.program.functions)?;
            match interpret::meets(original, &names, &result)? {
                false => Ok(()),
                true => Err("the mutant meets the contract there".to_string()),
            }
        };
        let (judged, _) = solvers.decide(&survives, &params, Polarity::Universal, breaks);
        match judged {
            Outcome::Refuted { .. } => {
                strength.caught += 1;
                continue;
            }
            Outcome::Proved { .. } => {}
            _ => {
                strength.undecided += 1;
                continue;
            }
        }
        // The witness must be an input on which the two really differ.
        let apart = |inputs: &Inputs| -> Result<(), String> {
            let names = interpret::admitted(original, inputs)?;
            let (ours, theirs) = (
                interpret::result(original, &names, &unit.program.functions)?,
                interpret::result(&mutant, &names, &unit.program.functions)?,
            );
            if ours == theirs {
                Err("the two agree there".to_string())
            } else {
                Ok(())
            }
        };
        let (separated, _) = solvers.decide(&differs, &named, Polarity::Existential, apart);
        match separated {
            Outcome::Refuted {
                evidence: Refutation::NoSuchInput,
            } => strength.equivalent += 1,
            Outcome::Proved {
                witness: Some(witness),
            } => {
                let inputs: Inputs = witness
                    .iter()
                    .filter(|(n, _)| params.contains(n))
                    .cloned()
                    .collect();
                let results = interpret::admitted(original, &inputs).and_then(|names| {
                    Ok((
                        interpret::result(original, &names, &unit.program.functions)?,
                        interpret::result(&mutant, &names, &unit.program.functions)?,
                    ))
                });
                match results {
                    Ok((ours, theirs)) => strength.survivors.push(Survivor {
                        change: describe(&change, unit),
                        inputs: unit.source_inputs(&inputs),
                        original: unit.source_value(&ours.source()),
                        mutant: unit.source_value(&theirs.source()),
                    }),
                    Err(_) => strength.undecided += 1,
                }
            }
            _ => strength.undecided += 1,
        }
    }
    Some(strength)
}

/// The strength of every function with an `ensures`, in source order.
pub(crate) fn strengths(units: &[Unit], solvers: &Solvers) -> Vec<Strength> {
    units
        .iter()
        .filter_map(|unit| judge(unit, solvers))
        .collect()
}
