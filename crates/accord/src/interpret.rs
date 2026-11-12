//! The interpreter: it runs `example`s, and replays each model a solver gives
//! so no counterexample or witness is taken on the solver's word alone.
//!
//! It evaluates in one of two modes. Code that runs is evaluated with each
//! arithmetic result checked against its width; specifications, and the
//! solvers' scripts, use mathematical integers.
//!
//! A call runs the callee's body, after checking its `requires`. A solver's
//! model of a caller is replayed twice when it doesn't reproduce as run:
//! once more with each call taking the value the model gives it, which tells
//! a counterexample to the code from one that rests on a callee's contract
//! being weaker than its body.

mod eval;
mod replay;
mod value;

use eval::{Env, Mode, body};
pub(crate) use replay::{Replay, replay};
pub(crate) use value::{Value, read};

use crate::Inputs;
use crate::syntax::{ClauseKind, Example, Function};

/// The first `requires` that doesn't hold, as written.
pub(crate) fn broken_requires(
    function: &Function,
    names: &[(String, Value)],
) -> Result<Option<String>, String> {
    let env = Env::plain(names, None, Mode::Math);
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        if !env.boolean(&clause.expr)? {
            return Ok(Some(clause.text.clone()));
        }
    }
    Ok(None)
}

/// Runs an example: `Ok` when the call gives the expected result, otherwise
/// why it doesn't. `functions` are those its function may call.
pub(crate) fn run(
    example: &Example,
    function: &Function,
    functions: &[Function],
) -> Result<(), String> {
    let constants = Env::plain(&[], None, Mode::Code);
    let names = example
        .args
        .iter()
        .map(|(label, expr)| Ok((label.clone().unwrap_or_default(), constants.eval(expr)?)))
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(clause) = broken_requires(function, &names)? {
        return Err(format!("the call breaks `{clause}`"));
    }
    let env = Env {
        names: &names,
        result: None,
        mode: Mode::Code,
        functions,
        assumed: None,
    };
    let got = body(function, &env)?;
    let want = constants.eval(&example.expected)?;
    if got == want {
        Ok(())
    } else {
        Err(format!("it gives {}, not {}", got.source(), want.source()))
    }
}

/// A solver's values for `function`'s parameters, read as values, provided
/// every `requires` holds of them.
pub(crate) fn admitted(function: &Function, inputs: &Inputs) -> Result<Names, String> {
    let names = function
        .params
        .iter()
        .map(|(name, ty)| {
            let raw = inputs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
                .ok_or(format!("no value for `{name}`"))?;
            read(raw, ty)
                .map(|value| (name.clone(), value))
                .ok_or(format!("`{raw}` isn't a value of `{name}`'s type"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(clause) = broken_requires(function, &names)? {
        return Err(format!("it breaks `{clause}`"));
    }
    Ok(names)
}

/// Parameter names with their values.
pub(crate) type Names = Vec<(String, Value)>;

/// `function`'s result for `names`, with mathematical integers; it may call
/// `functions`.
pub(crate) fn result(
    function: &Function,
    names: &Names,
    functions: &[Function],
) -> Result<Value, String> {
    let env = Env {
        names,
        result: None,
        mode: Mode::Math,
        functions,
        assumed: None,
    };
    body(function, &env)
}

/// Whether some `open when` of `function` covers `names`.
pub(crate) fn covered(function: &Function, names: &Names) -> Result<bool, String> {
    let env = Env::plain(names, None, Mode::Math);
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Open)
    {
        if env.boolean(&clause.expr)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether every `ensures` of `contract` holds of `result` for `names`.
pub(crate) fn meets(contract: &Function, names: &Names, result: &Value) -> Result<bool, String> {
    let env = Env::plain(names, Some(result), Mode::Math);
    for clause in contract
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures)
    {
        if !env.boolean(&clause.expr)? {
            return Ok(false);
        }
    }
    Ok(true)
}
