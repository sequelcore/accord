//! Comparing two revisions of a file's contracts. A function's contract is
//! its `requires` R and its `ensures` E. The new contract is at least as
//! strong as the old when it accepts every input the old one did
//! (`R_old ⇒ R_new`) and, on those inputs, promises everything the old one
//! did (`R_old ∧ E_new ⇒ E_old`). The solvers decide both directions, so a
//! loosened contract comes with the input, or the input and result, it newly
//! allows.
//!
//! Functions are matched by name, and a removed function whose definition
//! hash reappears under another name is a rename. Both revisions are
//! normalized, so renaming a parameter changes nothing.

mod output;
mod strength;

pub use output::{VERSION, json, text};
use strength::at_least_as_strong;

use crate::identity::{self, Unit};
use crate::syntax::{Function, Type};
use crate::{Diagnostic, Inputs, Solvers, check, syntax};

/// Which revision a diagnostic is about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    /// The old revision.
    Old,
    /// The new revision.
    New,
}

/// What the new revision newly allows, against the old one.
#[derive(Clone, Debug, PartialEq)]
pub enum Loosening {
    /// An input the old contract accepted and the new one doesn't.
    Rejects(Inputs),
    /// An input both accept, with a result the new contract allows and the
    /// old one ruled out.
    Allows(Inputs, String),
}

/// How one function's contract changed.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// The same contract, in meaning.
    Equivalent,
    /// The new contract promises more, or accepts more inputs, and nothing less.
    Stronger,
    /// The new contract promises less; the old one is stronger.
    Weaker(Loosening),
    /// Each is stronger somewhere; this is where the new one loosened.
    Incomparable(Loosening),
    /// The solvers didn't settle it.
    Undecided(String),
    /// Only in the new revision.
    Added,
    /// Only in the old revision: its guarantees are gone.
    Removed,
    /// The same definition under the old name.
    Renamed(String),
    /// The parameter or result types changed, so the contracts can't be compared.
    SignatureChanged,
    /// Neither revision has a `requires` or an `ensures`.
    NoContract,
}

/// One function's change.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionChange {
    /// The function's name in the new revision, or in the old if removed.
    pub name: String,
    /// How its contract changed.
    pub change: Change,
}

/// The comparison's overall result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// No contract got weaker.
    Kept,
    /// Some contract got weaker, incomparable, or was removed.
    Weakened,
    /// Nothing was proved weaker, but something couldn't be compared.
    Undecided,
}

/// Every function's change, and the overall verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// One per function in either revision: the new revision's order, then
    /// removed functions in the old revision's order.
    pub functions: Vec<FunctionChange>,
    /// The overall verdict.
    pub verdict: Verdict,
}

fn has_contract(function: &Function) -> bool {
    !function.clauses.is_empty()
}

/// The same signature, enums included, after normalization.
fn same_signature(old: &Unit, new: &Unit) -> bool {
    let (Some(a), Some(b)) = (old.program.functions.first(), new.program.functions.first()) else {
        return false;
    };
    let shape =
        |u: &Unit| -> Vec<usize> { u.program.enums.iter().map(|e| e.variants.len()).collect() };
    let types = |f: &Function| -> Vec<Type> { f.params.iter().map(|(_, t)| t.clone()).collect() };
    types(a) == types(b) && a.ret == b.ret && shape(old) == shape(new)
}

fn compare(old: &Unit, new: &Unit, solvers: &Solvers) -> Change {
    let (Some(a), Some(b)) = (old.program.functions.first(), new.program.functions.first()) else {
        return Change::SignatureChanged;
    };
    if !same_signature(old, new) {
        return Change::SignatureChanged;
    }
    if !has_contract(a) && !has_contract(b) {
        return Change::NoContract;
    }
    // The new revision's names read the loosenings in source terms.
    let new_is_strong = at_least_as_strong((b, new), (a, new), solvers);
    let old_is_strong = at_least_as_strong((a, new), (b, new), solvers);
    match (new_is_strong, old_is_strong) {
        (Err(reason), _) | (_, Err(reason)) => Change::Undecided(reason),
        (Ok(None), Ok(None)) => Change::Equivalent,
        (Ok(None), Ok(Some(_))) => Change::Stronger,
        (Ok(Some(loosening)), Ok(None)) => Change::Weaker(loosening),
        (Ok(Some(loosening)), Ok(Some(_))) => Change::Incomparable(loosening),
    }
}

fn parse(source: &str, side: Side) -> Result<syntax::Program, (Side, Diagnostic)> {
    let mut program = syntax::parse(source).map_err(|d| (side, d))?;
    check::check(&mut program, &[]).map_err(|d| (side, d))?;
    if let Some(machine) = program.machines.first() {
        return Err((
            side,
            Diagnostic::new(machine.line, "machines aren't compared yet"),
        ));
    }
    Ok(program)
}

/// Compares the contracts of two revisions of a file.
pub fn diff(old: &str, new: &str, solvers: &Solvers) -> Result<Comparison, (Side, Diagnostic)> {
    let (old_program, new_program) = (parse(old, Side::Old)?, parse(new, Side::New)?);
    let (old_units, new_units) = (identity::units(&old_program), identity::units(&new_program));
    let find = |units: &[Unit<'_>], name: &str| units.iter().position(|u| u.source.name == name);
    let mut functions = Vec::new();
    let mut matched_old = vec![false; old_units.len()];
    for unit in &new_units {
        let name = unit.source.name.clone();
        let change = match find(&old_units, &name) {
            Some(i) => {
                matched_old[i] = true;
                compare(&old_units[i], unit, solvers)
            }
            None => Change::Added,
        };
        functions.push(FunctionChange { name, change });
    }
    // A removed function whose definition reappears under a new name was renamed.
    for (_, old_unit) in old_units
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_old[*i])
    {
        let renamed = functions.iter_mut().find(|f| {
            f.change == Change::Added
                && new_units
                    .iter()
                    .any(|u| u.source.name == f.name && u.hash == old_unit.hash)
        });
        match renamed {
            Some(f) => f.change = Change::Renamed(old_unit.source.name.clone()),
            None => functions.push(FunctionChange {
                name: old_unit.source.name.clone(),
                change: Change::Removed,
            }),
        }
    }
    let weakened = functions.iter().any(|f| {
        matches!(
            f.change,
            Change::Weaker(_) | Change::Incomparable(_) | Change::Removed
        )
    });
    let undecided = functions
        .iter()
        .any(|f| matches!(f.change, Change::Undecided(_) | Change::SignatureChanged));
    let verdict = match (weakened, undecided) {
        (true, _) => Verdict::Weakened,
        (false, true) => Verdict::Undecided,
        (false, false) => Verdict::Kept,
    };
    Ok(Comparison { functions, verdict })
}
