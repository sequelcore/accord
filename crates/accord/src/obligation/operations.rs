//! The operations in code that carry obligations, arithmetic and calls,
//! each with the conditions under which it runs.

use super::smt::{all, has_requires, requires_of, term};
use crate::syntax::{Body, Expr, Function, Program};

/// A condition that must have a given value for code to run; `None` is a
/// row's `otherwise`, which always holds.
#[derive(Clone, Copy)]
pub(crate) struct Guard<'a> {
    pub condition: Option<&'a Expr>,
    pub holds: bool,
}
pub(super) fn guard_term(guard: &Guard) -> String {
    let c = guard.condition.map_or("true".to_string(), term);
    if guard.holds { c } else { format!("(not {c})") }
}
/// One arithmetic operation or call in code, with the conditions under which
/// it runs.
pub(super) struct Operation<'a> {
    pub(super) expr: &'a Expr,
    pub(super) guards: Vec<Guard<'a>>,
}
/// Every arithmetic operation and call in `expr`, in the order they run:
/// operands and arguments first. Only the branch of an `if` that runs is
/// guarded by its condition; both operands of `and` and `or` run, whatever
/// the other's value.
fn operations<'a>(expr: &'a Expr, guards: &[Guard<'a>], out: &mut Vec<Operation<'a>>) {
    let push = |out: &mut Vec<Operation<'a>>| {
        out.push(Operation {
            expr,
            guards: guards.to_vec(),
        });
    };
    match expr {
        Expr::Arith(_, left, right, _) => {
            operations(left, guards, out);
            operations(right, guards, out);
            push(out);
        }
        Expr::Neg(inner, _) => {
            operations(inner, guards, out);
            push(out);
        }
        Expr::Call(_, args, _) => {
            for (_, arg) in args {
                operations(arg, guards, out);
            }
            push(out);
        }
        Expr::Not(inner) => operations(inner, guards, out),
        Expr::Binary(_, left, right) => {
            operations(left, guards, out);
            operations(right, guards, out);
        }
        Expr::If(condition, then, otherwise) => {
            operations(condition, guards, out);
            let guard = |holds| Guard {
                condition: Some(condition),
                holds,
            };
            operations(then, &[guards, &[guard(true)]].concat(), out);
            operations(otherwise, &[guards, &[guard(false)]].concat(), out);
        }
        Expr::List(items, ..) => {
            for item in items {
                operations(item, guards, out);
            }
        }
        Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
}
/// That where `op` runs, its callee's `requires` holds; `None` for an
/// operation that isn't a call to a function with a `requires`.
pub(super) fn assumption(program: &Program, op: &Operation) -> Option<String> {
    if !matches!(op.expr, Expr::Call(..)) || !has_requires(program, op.expr) {
        return None;
    }
    let guards = all("and", op.guards.iter().map(guard_term).collect(), "true");
    Some(format!("(=> {guards} {})", requires_of(program, op.expr)))
}
/// Every operation in `function`'s body, in the order they run; a decision
/// table's rows run in order, each only when no earlier row's condition holds.
pub(super) fn body_operations(function: &Function) -> Vec<Operation<'_>> {
    let mut ops = Vec::new();
    match &function.body {
        Body::Expr(body) => operations(body, &[], &mut ops),
        Body::Decide(rows, _) => {
            for (index, row) in rows.iter().enumerate() {
                let mut reaches: Vec<Guard> = rows[..index]
                    .iter()
                    .map(|earlier| Guard {
                        condition: earlier.condition.as_ref(),
                        holds: false,
                    })
                    .collect();
                if let Some(c) = &row.condition {
                    operations(c, &reaches, &mut ops);
                }
                reaches.push(Guard {
                    condition: row.condition.as_ref(),
                    holds: true,
                });
                operations(&row.result, &reaches, &mut ops);
            }
        }
    }
    ops
}
/// Every call in `function`'s body, in the order they run.
pub(crate) fn calls_in<'a>(function: &'a Function, out: &mut Vec<&'a Expr>) {
    out.extend(
        body_operations(function)
            .into_iter()
            .map(|op| op.expr)
            .filter(|e| matches!(e, Expr::Call(..))),
    );
}
