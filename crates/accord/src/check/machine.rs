//! A machine's state, inits, events, observers and invariants, and the runs
//! its examples give. Checking desugars a machine into functions: each member
//! takes the machine's fields as its first parameters and carries the
//! invariants as clauses, and each `next` gives every field, in order.

use super::calls::{Signature, check_recursion, ordered};
use super::expr::Scope;
use super::{Types, check_clauses, check_function, check_type, type_name};
use crate::Diagnostic;
use crate::syntax::{Body, Expr, Function, Machine, MachineExample, Member, Program, Role, Type};

/// Checks `machine` against the program's `types` and `functions`, whose
/// signatures are `globals`, and desugars it.
pub(super) fn check(
    types: Types<'_>,
    globals: &[Signature],
    functions: &[Function],
    machine: &mut Machine,
) -> Result<(), Diagnostic> {
    let state = machine.state.clone();
    check_state(types, machine)?;
    if machine.inits.is_empty() {
        return Err(Diagnostic::new(
            machine.line,
            format!("`{}` needs an `init`: a way to start", machine.name),
        ));
    }
    check_names(machine, functions)?;
    for member in machine
        .inits
        .iter()
        .chain(&machine.events)
        .chain(&machine.observers)
    {
        let types_used = member.params.iter().map(|(_, ty)| ty).chain([&member.ret]);
        if let Some(ty) = types_used.into_iter().find(|ty| types.structured(ty)) {
            return Err(Diagnostic::new(
                member.line,
                format!(
                    "`{}` takes or gives a `{}`: a machine can't hold records or variants with fields yet",
                    member.name,
                    type_name(ty)
                ),
            ));
        }
        for (param, ty) in &member.params {
            check_type(types, ty, member.line)?;
            if param == "next" || state.iter().any(|(field, _)| field == param) {
                return Err(Diagnostic::new(
                    member.line,
                    format!("a parameter can't be named `{param}`: it is a field or `next`"),
                ));
            }
        }
    }
    let mut signatures = globals.to_vec();
    signatures.extend(machine.observers.iter().map(|observer| Signature {
        name: observer.name.clone(),
        params: observer.params.clone(),
        ret: observer.ret.clone(),
        state: state.clone(),
    }));
    for init in &mut machine.inits {
        // An init doesn't read the state: its fields are placeholders, added
        // once its body is checked.
        check_clauses(types, init)?;
        check_body(types, globals, init, &state)?;
        with_state(init, &state);
    }
    for event in &mut machine.events {
        with_state(event, &state);
        check_clauses(types, event)?;
        check_body(types, &signatures, event, &state)?;
    }
    for observer in &mut machine.observers {
        with_state(observer, &state);
        check_function(types, &signatures, observer)?;
    }
    check_recursion(&Program {
        enums: Vec::new(),
        records: Vec::new(),
        functions: functions
            .iter()
            .chain(&machine.observers)
            .cloned()
            .collect(),
        machines: Vec::new(),
        examples: Vec::new(),
        machine_examples: Vec::new(),
    })?;
    for invariant in &mut machine.invariants {
        let scope = Scope {
            types,
            params: &state,
            result: None,
            spec: true,
            functions: None,
            indexing: false,
            bound: false,
        };
        scope.condition(&mut invariant.expr)?;
    }
    let invariants = machine.invariants.clone();
    for member in machine
        .inits
        .iter_mut()
        .chain(&mut machine.events)
        .chain(&mut machine.observers)
    {
        member.clauses.extend(invariants.iter().cloned());
    }
    Ok(())
}

fn check_state(types: Types<'_>, machine: &Machine) -> Result<(), Diagnostic> {
    for (index, (field, ty)) in machine.state.iter().enumerate() {
        check_type(types, ty, machine.line)?;
        if !matches!(ty, Type::Bool | Type::Int(_) | Type::Enum(_)) || types.structured(ty) {
            return Err(Diagnostic::new(
                machine.line,
                format!(
                    "`{field}` is a `{}`: a state holds fixed-width integers, enums and `Bool`s",
                    type_name(ty)
                ),
            ));
        }
        if field == "next" {
            return Err(Diagnostic::new(
                machine.line,
                "a field can't be named `next`",
            ));
        }
        if machine.state[..index]
            .iter()
            .any(|(other, _)| other == field)
        {
            return Err(Diagnostic::new(
                machine.line,
                format!("`{field}` is a field twice"),
            ));
        }
    }
    Ok(())
}

/// Every init, event and observer has a name of its own, and an observer
/// isn't named like a function it could be mistaken for.
fn check_names(machine: &Machine, functions: &[Function]) -> Result<(), Diagnostic> {
    let members: Vec<&Function> = machine.members().collect();
    for (index, member) in members.iter().enumerate() {
        let observer = member.machine.is_some_and(|m| m.role == Role::Observer);
        if members[..index]
            .iter()
            .any(|other| other.name == member.name)
            || (observer && functions.iter().any(|f| f.name == member.name))
        {
            return Err(Diagnostic::new(
                member.line,
                format!("`{}` is declared more than once", member.name),
            ));
        }
    }
    Ok(())
}

/// Puts the machine's fields before `member`'s own parameters.
fn with_state(member: &mut Function, state: &[(String, Type)]) {
    let mut params = state.to_vec();
    params.append(&mut member.params);
    member.params = params;
    if let Some(machine) = &mut member.machine {
        *machine = Member {
            role: machine.role,
            state: state.len(),
        };
    }
}

/// Checks an init's or event's body, which ends in `next` through `if` or a
/// decision table. An init's scope has no fields.
fn check_body(
    types: Types<'_>,
    signatures: &[Signature],
    member: &mut Function,
    state: &[(String, Type)],
) -> Result<(), Diagnostic> {
    let init = member.machine.is_some_and(|m| m.role == Role::Init);
    let scope = Scope {
        types,
        params: &member.params,
        result: None,
        spec: false,
        functions: Some(signatures),
        indexing: true,
        bound: false,
    };
    let name = member.name.clone();
    match &mut member.body {
        Body::Expr(body) => step(&scope, state, init.then_some(&name), body),
        Body::Decide(rows, _) => {
            for row in rows {
                if let Some(condition) = &mut row.condition {
                    scope.condition(condition)?;
                }
                step(&scope, state, init.then_some(&name), &mut row.result)?;
            }
            Ok(())
        }
    }
}

/// Checks the `if`s and the `next` that end in a body, and gives each `next`
/// every field. An init (`init` names it) must give each field; an event
/// keeps the value of any it leaves out.
fn step(
    scope: &Scope,
    state: &[(String, Type)],
    init: Option<&String>,
    expr: &mut Expr,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::If(condition, then, otherwise) => {
            scope.condition(condition)?;
            step(scope, state, init, then)?;
            step(scope, state, init, otherwise)
        }
        Expr::Next(fields, line) => {
            let line = *line;
            let mut values: Vec<Option<Expr>> = state.iter().map(|_| None).collect();
            for (name, value) in std::mem::take(fields) {
                let Some(index) = state.iter().position(|(field, _)| *field == name) else {
                    return Err(Diagnostic::new(
                        line,
                        format!("`{name}` isn't a field of the state"),
                    ));
                };
                if values[index].replace(value).is_some() {
                    return Err(Diagnostic::new(
                        line,
                        format!("`next` gives `{name}` twice"),
                    ));
                }
            }
            for (value, (field, ty)) in values.into_iter().zip(state) {
                let mut value = match (value, init) {
                    (Some(value), _) => value,
                    (None, Some(init)) => {
                        return Err(Diagnostic::new(
                            line,
                            format!("`{init}` leaves `{field}` out: an init gives every field"),
                        ));
                    }
                    (None, None) => Expr::Name(field.clone(), line),
                };
                scope.value(
                    &mut value,
                    ty,
                    &format!("`{field}` needs a `{}`", type_name(ty)),
                )?;
                fields.push((field.clone(), value));
            }
            Ok(())
        }
        other => Err(Diagnostic::new(
            other.line(),
            "an init or event ends in `next { … }`",
        )),
    }
}

/// Checks an example's run of `machine`: an init, then events, each with
/// arguments put in the order of its parameters, and the condition it gives.
pub(super) fn check_run(
    types: Types<'_>,
    machines: &[Machine],
    run: &mut MachineExample,
) -> Result<(), Diagnostic> {
    let Some(machine) = machines.iter().find(|m| m.name == run.machine) else {
        return Err(Diagnostic::new(
            run.line,
            format!("unknown machine `{}`", run.machine),
        ));
    };
    if run.steps.is_empty() {
        return Err(Diagnostic::new(
            run.line,
            "an example runs an init, then events",
        ));
    }
    let constants = Scope {
        types,
        params: &[],
        result: None,
        spec: false,
        functions: None,
        indexing: false,
        bound: false,
    };
    for (index, step) in run.steps.iter_mut().enumerate() {
        let line = step.line();
        let Expr::Call(name, args, _) = step else {
            return Err(Diagnostic::new(
                line,
                "each step is an init's or an event's call",
            ));
        };
        let (pool, kind) = if index == 0 {
            (&machine.inits, "an init")
        } else {
            (&machine.events, "an event")
        };
        let Some(member) = pool.iter().find(|f| f.name == *name) else {
            return Err(Diagnostic::new(
                line,
                format!("`{name}` isn't {kind} of `{}`", machine.name),
            ));
        };
        let own = &member.params[machine.state.len()..];
        let given = ordered(name, own, std::mem::take(args), line)?;
        for ((param, ty), mut expr) in own.iter().zip(given) {
            constants.value(
                &mut expr,
                ty,
                &format!("`{param}` needs a `{}`", type_name(ty)),
            )?;
            args.push((Some(param.clone()), expr));
        }
    }
    let scope = Scope {
        types,
        params: &machine.state,
        result: None,
        spec: true,
        functions: None,
        indexing: false,
        bound: false,
    };
    scope.condition(&mut run.gives)
}
