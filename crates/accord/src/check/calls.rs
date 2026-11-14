//! Calls between functions: their arguments, the functions a body calls,
//! and the recursion that isn't supported yet.

use super::expr::Scope;
use super::type_name;
use crate::Diagnostic;
use crate::syntax::{Body, Expr, Function, Program, Type};

/// What a call needs to know about the function it calls.
#[derive(Clone)]
pub(super) struct Signature {
    pub name: String,
    /// The parameters a call gives.
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    /// The machine fields an observer reads, which a call passes on as its
    /// first arguments; empty for a function.
    pub state: Vec<(String, Type)>,
}

impl Scope<'_> {
    /// Checks a call's arguments against the function it names, puts them in
    /// the order of its parameters, each labelled, and gives its result type.
    pub(super) fn call(
        &self,
        name: &str,
        args: &mut Vec<(Option<String>, Expr)>,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let Some(functions) = self.functions else {
            return Err(Diagnostic::new(
                line,
                if self.spec {
                    "a specification can't call a function yet"
                } else {
                    "an example's arguments can't call a function"
                },
            ));
        };
        let Some(callee) = functions.iter().find(|f| f.name == name) else {
            return Err(Diagnostic::new(line, format!("unknown function `{name}`")));
        };
        let ordered = ordered(name, &callee.params, std::mem::take(args), line)?;
        for (field, _) in &callee.state {
            args.push((Some(field.clone()), Expr::Name(field.clone(), line)));
        }
        for ((param, ty), mut expr) in callee.params.iter().zip(ordered) {
            self.value(
                &mut expr,
                ty,
                &format!("`{param}` needs a `{}`", type_name(ty)),
            )?;
            args.push((Some(param.clone()), expr));
        }
        Ok(callee.ret.clone())
    }
}

/// A call's arguments in the order of `params`, from arguments that are
/// labelled, or a single one that may not be.
pub(super) fn ordered(
    function: &str,
    params: &[(String, Type)],
    args: Vec<(Option<String>, Expr)>,
    line: usize,
) -> Result<Vec<Expr>, Diagnostic> {
    let count = params.len();
    if args.len() != count {
        return Err(Diagnostic::new(
            line,
            format!("`{function}` takes {count} arguments"),
        ));
    }
    if count > 1 && args.iter().any(|(label, _)| label.is_none()) {
        return Err(Diagnostic::new(
            line,
            format!("label every argument: `{function}` has more than one parameter"),
        ));
    }
    let mut ordered: Vec<Option<Expr>> = vec![None; count];
    for (label, expr) in args {
        let index = match &label {
            None => 0,
            Some(label) => params
                .iter()
                .position(|(name, _)| name == label)
                .ok_or_else(|| {
                    Diagnostic::new(line, format!("`{function}` has no parameter `{label}`"))
                })?,
        };
        if ordered[index].replace(expr).is_some() {
            return Err(Diagnostic::new(
                line,
                format!("`{}` is given twice", params[index].0),
            ));
        }
    }
    params
        .iter()
        .zip(ordered)
        .map(|((name, _), expr)| {
            expr.ok_or_else(|| Diagnostic::new(line, format!("`{name}` is missing")))
        })
        .collect()
}

/// The names of the functions `expr` calls, inner calls first, each once.
pub(crate) fn callees(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Call(name, args, _) => {
            for (_, arg) in args {
                callees(arg, out);
            }
            if !out.contains(name) {
                out.push(name.clone());
            }
        }
        Expr::Not(inner) | Expr::Neg(inner, _) => callees(inner, out),
        Expr::Construct(..)
        | Expr::Record(..)
        | Expr::Field(..)
        | Expr::Match(..)
        | Expr::Is(..) => expr.children().into_iter().for_each(|c| callees(c, out)),
        Expr::Binary(_, left, right) | Expr::Arith(_, left, right, _) => {
            callees(left, out);
            callees(right, out);
        }
        Expr::If(condition, then, otherwise) => {
            callees(condition, out);
            callees(then, out);
            callees(otherwise, out);
        }
        Expr::List(items, ..) => items.iter().for_each(|item| callees(item, out)),
        Expr::Quantified(_, _, range, condition, _) => {
            callees(range.list(), out);
            callees(condition, out);
        }
        Expr::Length(list, _) => callees(list, out),
        Expr::Next(fields, _) => fields.iter().for_each(|(_, value)| callees(value, out)),
        Expr::Index(list, index, _) => {
            callees(list, out);
            callees(index, out);
        }
        Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
}

/// The names of the functions `function`'s body calls, each once.
pub(crate) fn body_callees(function: &Function) -> Vec<String> {
    let mut out = Vec::new();
    match &function.body {
        Body::Expr(body) => callees(body, &mut out),
        Body::Decide(rows, _) => {
            for row in rows {
                if let Some(condition) = &row.condition {
                    callees(condition, &mut out);
                }
                callees(&row.result, &mut out);
            }
        }
    }
    out
}

/// Rejects a function that calls itself, directly or through others, since
/// termination isn't proved yet.
pub(super) fn check_recursion(program: &Program) -> Result<(), Diagnostic> {
    for function in &program.functions {
        let mut seen: Vec<String> = Vec::new();
        let mut pending = body_callees(function);
        while let Some(name) = pending.pop() {
            if name == function.name {
                return Err(Diagnostic::new(
                    function.line,
                    format!(
                        "`{}` calls itself, directly or through another function: recursion isn't supported yet",
                        function.name
                    ),
                ));
            }
            if seen.contains(&name) {
                continue;
            }
            if let Some(callee) = program.functions.iter().find(|f| f.name == name) {
                pending.extend(body_callees(callee));
            }
            seen.push(name);
        }
    }
    Ok(())
}
