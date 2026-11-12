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

pub use output::{VERSION, json, text};

use crate::identity::{self, Unit};
use crate::interpret;
use crate::obligation::{self, Polarity};
use crate::syntax::{ClauseKind, Function, Type};
use crate::{Diagnostic, Inputs, Outcome, Solvers, check, syntax};

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

fn condition(function: &Function, kind: ClauseKind) -> String {
    obligation::all(
        "and",
        function
            .clauses
            .iter()
            .filter(|c| c.kind == kind)
            .map(|c| obligation::term(&c.expr))
            .collect(),
        "true",
    )
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

/// Whether `stronger` is at least as strong as `weaker`: `Ok(None)` if so,
/// `Ok(Some(loosening))` where it isn't, `Err` if undecided.
fn at_least_as_strong(
    stronger: (&Function, &Unit),
    weaker: (&Function, &Unit),
    solvers: &Solvers,
) -> Result<Option<Loosening>, String> {
    let (s, unit) = stronger;
    let (w, _) = weaker;
    let datatypes = obligation::datatypes(&unit.program);
    let mut prelude = format!("(set-option :produce-models true)\n(set-logic ALL)\n{datatypes}");
    for (name, ty) in &s.params {
        prelude += &format!("(declare-const {name} {})\n", obligation::sort(ty));
        if let Type::Int(width) = ty {
            prelude += &format!("(assert {})\n", obligation::in_range(name, *width));
        }
    }
    let params: Vec<String> = s.params.iter().map(|(n, _)| n.clone()).collect();
    let (w_requires, s_requires) = (
        condition(w, ClauseKind::Requires),
        condition(s, ClauseKind::Requires),
    );

    // Every input the weaker contract accepts, the stronger one accepts too.
    let accepts = format!(
        "{prelude}(assert (and {w_requires} (not {s_requires})))\n{}",
        obligation::ending(&params)
    );
    let rejects = |inputs: &Inputs| -> Result<(), String> {
        interpret::admitted(w, inputs)?;
        match interpret::admitted(s, inputs) {
            Err(_) => Ok(()),
            Ok(_) => Err("the stronger contract accepts it too".to_string()),
        }
    };
    match solvers
        .decide(&accepts, &params, Polarity::Universal, rejects)
        .0
    {
        Outcome::Proved { .. } => {}
        Outcome::Refuted {
            evidence: crate::Refutation::Counterexample(inputs),
        } => return Ok(Some(Loosening::Rejects(unit.source_inputs(&inputs)))),
        Outcome::Undecided { reason } | Outcome::Unsupported { reason } => return Err(reason),
        Outcome::Refuted { .. } | Outcome::WeakCallee { .. } => {
            return Err("no counterexample".to_string());
        }
    }

    // On those inputs, the stronger contract promises what the weaker does.
    let ret = obligation::sort(&s.ret);
    let mut result = format!("(declare-const result {ret})\n");
    if let Type::Int(width) = &s.ret {
        result += &format!("(assert {})\n", obligation::in_range("result", *width));
    }
    let mut named = params.clone();
    named.push("result".to_string());
    let promises = format!(
        "{prelude}{result}(assert (and {w_requires} {} (not {})))\n{}",
        condition(s, ClauseKind::Ensures),
        condition(w, ClauseKind::Ensures),
        obligation::ending(&named)
    );
    let allows = |inputs: &Inputs| -> Result<(), String> {
        let names = interpret::admitted(w, inputs)?;
        let raw = inputs
            .iter()
            .find(|(n, _)| n == "result")
            .map(|(_, v)| v.as_str())
            .ok_or("no value for `result`")?;
        let value = interpret::read(raw, &s.ret).ok_or("an unreadable result")?;
        match (
            interpret::meets(s, &names, &value)?,
            interpret::meets(w, &names, &value)?,
        ) {
            (true, false) => Ok(()),
            _ => Err("the contracts agree there".to_string()),
        }
    };
    match solvers
        .decide(&promises, &named, Polarity::Universal, allows)
        .0
    {
        Outcome::Proved { .. } => Ok(None),
        Outcome::Refuted {
            evidence: crate::Refutation::Counterexample(inputs),
        } => {
            let result = inputs
                .iter()
                .find(|(n, _)| n == "result")
                .map(|(_, v)| unit.source_value(v))
                .unwrap_or_default();
            let inputs: Inputs = inputs.into_iter().filter(|(n, _)| n != "result").collect();
            Ok(Some(Loosening::Allows(unit.source_inputs(&inputs), result)))
        }
        Outcome::Undecided { reason } | Outcome::Unsupported { reason } => Err(reason),
        Outcome::Refuted { .. } | Outcome::WeakCallee { .. } => {
            Err("no counterexample".to_string())
        }
    }
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
