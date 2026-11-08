//! Name and type checks that run before any obligation is generated.

use crate::Diagnostic;
use crate::syntax::{ClauseKind, Expr, Function, Op, Type};

fn type_of(
    expr: &Expr,
    names: &[(String, Type)],
    result: Option<&Type>,
) -> Result<Type, Diagnostic> {
    match expr {
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::Name(name, line) => {
            if name == "result" {
                return result.cloned().ok_or_else(|| {
                    Diagnostic::new(*line, "`result` is only available in `ensures`")
                });
            }
            names
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| Diagnostic::new(*line, format!("unknown name `{name}`")))
        }
        Expr::Not(inner) => bool_operand(inner, names, result),
        Expr::Binary(op, left, right) => {
            let l = type_of(left, names, result)?;
            let r = type_of(right, names, result)?;
            match op {
                Op::Eq | Op::Ne if l == r => Ok(Type::Bool),
                Op::Eq | Op::Ne => Err(Diagnostic::new(
                    line_of(expr),
                    "both sides of a comparison need the same type",
                )),
                Op::And | Op::Or | Op::Implies => {
                    bool_operand(left, names, result)?;
                    bool_operand(right, names, result)
                }
            }
        }
        Expr::If(condition, then, otherwise) => {
            bool_operand(condition, names, result)?;
            let a = type_of(then, names, result)?;
            let b = type_of(otherwise, names, result)?;
            if a == b {
                Ok(a)
            } else {
                Err(Diagnostic::new(
                    line_of(expr),
                    "both branches of `if` need the same type",
                ))
            }
        }
    }
}

fn bool_operand(
    expr: &Expr,
    names: &[(String, Type)],
    result: Option<&Type>,
) -> Result<Type, Diagnostic> {
    match type_of(expr, names, result)? {
        Type::Bool => Ok(Type::Bool),
    }
}

fn line_of(expr: &Expr) -> usize {
    match expr {
        Expr::Name(_, line) => *line,
        Expr::Not(inner) => line_of(inner),
        Expr::Binary(_, left, _) | Expr::If(left, _, _) => line_of(left),
        Expr::Bool(_) => 0,
    }
}

/// Checks names and types in every function.
pub(crate) fn check(functions: &[Function]) -> Result<(), Diagnostic> {
    for function in functions {
        for clause in &function.clauses {
            let result = (clause.kind == ClauseKind::Ensures).then_some(&function.ret);
            if type_of(&clause.expr, &function.params, result)? != Type::Bool {
                return Err(Diagnostic::new(
                    clause.line,
                    "a clause must be a `Bool` condition",
                ));
            }
        }
        if type_of(&function.body, &function.params, None)? != function.ret {
            return Err(Diagnostic::new(
                line_of(&function.body),
                "the body's type isn't the function's return type",
            ));
        }
    }
    Ok(())
}
