//! Whether one contract is at least as strong as another, decided by the
//! solvers, with what the weaker one newly allows where it isn't.

use super::Loosening;
use crate::identity::Unit;
use crate::interpret;
use crate::obligation::{self, Polarity};
use crate::syntax::{ClauseKind, Function, Type};
use crate::{Inputs, Outcome, Solvers};

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

/// Whether `stronger` is at least as strong as `weaker`: `Ok(None)` if so,
/// `Ok(Some(loosening))` where it isn't, `Err` if undecided.
pub(super) fn at_least_as_strong(
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
