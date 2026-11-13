//! Running a machine: an init or an event in code, the invariants of a state,
//! and an example's run.

use super::eval::{Env, Mode};
use super::{Names, Value, broken, execute};
use crate::syntax::{ClauseKind, Expr, Function, Machine, MachineExample};

/// The state `member` ends in, as the value of each field in order, when run
/// in code on `names`: its own parameters, and for an event the state.
pub(crate) fn transition(
    member: &Function,
    names: &Names,
    functions: &[Function],
) -> Result<Vec<Value>, String> {
    match execute(member, names, functions)? {
        Value::List(state) => Ok(state),
        _ => Err("a transition ends in a state".to_string()),
    }
}

/// The fields of `member`'s machine, each with its value in `state`.
pub(crate) fn fields(member: &Function, state: &[Value]) -> Names {
    let count = member.machine.map_or(0, |m| m.state);
    member.params[..count]
        .iter()
        .map(|(name, _)| name.clone())
        .zip(state.iter().cloned())
        .collect()
}

/// The first invariant, or helper invariant, that `state` breaks, as written.
pub(crate) fn broken_invariant(
    member: &Function,
    state: &[Value],
) -> Result<Option<String>, String> {
    broken(member, &fields(member, state), ClauseKind::Invariant)
}

/// Runs an example: its init, then its events, each in code, then whether the
/// condition it gives holds of the state it ends in.
pub(crate) fn run(
    example: &MachineExample,
    machine: &Machine,
    functions: &[Function],
) -> Result<(), String> {
    let constants = Env::plain(&[], None, Mode::Code);
    let mut state: Option<Vec<Value>> = None;
    for (index, step) in example.steps.iter().enumerate() {
        let Expr::Call(name, args, site) = step else {
            return Err("a step is a call".to_string());
        };
        let pool = if index == 0 {
            &machine.inits
        } else {
            &machine.events
        };
        let member = pool
            .iter()
            .find(|f| f.name == *name)
            .ok_or(format!("`{name}` isn't a step of `{}`", machine.name))?;
        let mut names = match &state {
            Some(state) => fields(member, state),
            None => Vec::new(),
        };
        for (label, arg) in args {
            names.push((label.clone().unwrap_or_default(), constants.eval(arg)?));
        }
        state = Some(
            transition(member, &names, functions)
                .map_err(|why| format!("`{}`: {why}", site.text))?,
        );
    }
    let state = state.ok_or("the example runs nothing")?;
    let names: Names = machine
        .state
        .iter()
        .map(|(field, _)| field.clone())
        .zip(state)
        .collect();
    if Env::plain(&names, None, Mode::Math).boolean(&example.gives)? {
        Ok(())
    } else {
        let after: Vec<String> = names
            .iter()
            .map(|(field, value)| format!("{field} = {}", value.source()))
            .collect();
        Err(format!(
            "it doesn't hold after the run: {}",
            after.join(", ")
        ))
    }
}
