//! The operations that carry obligations, each with the conditions under
//! which it is evaluated and the quantifiers around it: arithmetic, calls
//! and indexing in code, and indexing in an `ensures`.

use super::calls::{has_requires, requires_of};
use super::lists::{at_index, in_bounds, index_name};
use super::smt::{all, in_range, term};
use crate::syntax::{Arm, Body, Expr, Function, Op, Owner, Program};

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
/// One operation, with the conditions under which it is evaluated and the
/// quantifiers it is inside, outermost first.
#[derive(Clone)]
pub(super) struct Operation<'a> {
    pub(super) expr: &'a Expr,
    pub(super) guards: Vec<Guard<'a>>,
    pub(super) bound: Vec<&'a Expr>,
}
/// `guards` and the condition under which `arm` runs: its variant is the
/// value matched.
fn arm_guards<'a>(guards: &[Guard<'a>], arm: &'a Arm) -> Vec<Guard<'a>> {
    let mut out = guards.to_vec();
    out.push(Guard {
        condition: arm.test.as_ref(),
        holds: true,
    });
    out
}
/// Whether an operation is an arithmetic operation, a call or an index.
fn carries_obligation(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Arith(..) | Expr::Neg(..) | Expr::Call(..) | Expr::Index(..)
    )
}
/// Every arithmetic operation, call and index in `expr`, in the order they
/// run: operands and arguments first. Only the branch of an `if` that runs is
/// guarded by its condition; both operands of `and` and `or` run, whatever
/// the other's value, and a quantifier's condition runs for every element.
fn operations<'a>(
    expr: &'a Expr,
    guards: &[Guard<'a>],
    bound: &[&'a Expr],
    out: &mut Vec<Operation<'a>>,
) {
    let each = |inner: &'a Expr, out: &mut Vec<Operation<'a>>| {
        operations(inner, guards, bound, out);
    };
    match expr {
        Expr::Arith(_, left, right, _)
        | Expr::Binary(_, left, right)
        | Expr::Index(left, right, _) => {
            each(left, out);
            each(right, out);
        }
        Expr::Neg(inner, _) | Expr::Not(inner) | Expr::Length(inner, _) => each(inner, out),
        Expr::Call(_, args, _) => args.iter().for_each(|(_, arg)| each(arg, out)),
        Expr::List(items, ..) => items.iter().for_each(|item| each(item, out)),
        Expr::Next(fields, _) => fields.iter().for_each(|(_, value)| each(value, out)),
        Expr::If(condition, then, otherwise) => {
            each(condition, out);
            let guard = |holds| Guard {
                condition: Some(condition),
                holds,
            };
            operations(then, &[guards, &[guard(true)]].concat(), bound, out);
            operations(otherwise, &[guards, &[guard(false)]].concat(), bound, out);
        }
        Expr::Quantified(_, _, range, condition, _) => {
            each(range.list(), out);
            operations(condition, guards, &[bound, &[expr]].concat(), out);
        }
        Expr::Construct(_, _, args, _) => args.iter().for_each(|(_, arg)| each(arg, out)),
        Expr::Record(_, fields, _) => fields.iter().for_each(|(_, value)| each(value, out)),
        Expr::Field(value, Some(Owner::Record(_)), ..) => each(value, out),
        Expr::Match(value, arms, _) => {
            each(value, out);
            for arm in arms {
                operations(&arm.result, &arm_guards(guards, arm), bound, out);
            }
        }
        // The value an arm reads a field of, or tests, is the one its
        // `match` already evaluated.
        Expr::Field(..)
        | Expr::Is(..)
        | Expr::Bool(_)
        | Expr::Int(..)
        | Expr::Text(..)
        | Expr::Name(..)
        | Expr::Variant(..) => {}
    }
    if carries_obligation(expr) {
        out.push(Operation {
            expr,
            guards: guards.to_vec(),
            bound: bound.to_vec(),
        });
    }
}
/// Every index in a specification, with the conditions under which it is
/// evaluated: the right of `implies` and `and` where the left holds, the
/// right of `or` where it doesn't, the branch of an `if` that runs, and a
/// quantifier's condition for each index in range.
pub(super) fn spec_indexes<'a>(
    expr: &'a Expr,
    guards: &[Guard<'a>],
    bound: &[&'a Expr],
    out: &mut Vec<Operation<'a>>,
) {
    let after = |left: &'a Expr, holds| {
        [
            guards,
            &[Guard {
                condition: Some(left),
                holds,
            }],
        ]
        .concat()
    };
    match expr {
        Expr::Binary(op @ (Op::Implies | Op::And | Op::Or), left, right) => {
            spec_indexes(left, guards, bound, out);
            spec_indexes(right, &after(left, *op != Op::Or), bound, out);
        }
        Expr::If(condition, then, otherwise) => {
            spec_indexes(condition, guards, bound, out);
            spec_indexes(then, &after(condition, true), bound, out);
            spec_indexes(otherwise, &after(condition, false), bound, out);
        }
        Expr::Quantified(_, _, range, condition, _) => {
            spec_indexes(range.list(), guards, bound, out);
            spec_indexes(condition, guards, &[bound, &[expr]].concat(), out);
        }
        Expr::Arith(_, left, right, _)
        | Expr::Binary(_, left, right)
        | Expr::Index(left, right, _) => {
            spec_indexes(left, guards, bound, out);
            spec_indexes(right, guards, bound, out);
        }
        Expr::Neg(inner, _) | Expr::Not(inner) | Expr::Length(inner, _) => {
            spec_indexes(inner, guards, bound, out);
        }
        Expr::List(items, ..) => items
            .iter()
            .for_each(|i| spec_indexes(i, guards, bound, out)),
        Expr::Call(_, args, _) | Expr::Construct(_, _, args, _) => args
            .iter()
            .for_each(|(_, a)| spec_indexes(a, guards, bound, out)),
        Expr::Record(_, fields, _) => fields
            .iter()
            .for_each(|(_, value)| spec_indexes(value, guards, bound, out)),
        Expr::Field(value, Some(Owner::Record(_)), ..) => spec_indexes(value, guards, bound, out),
        Expr::Match(value, arms, _) => {
            spec_indexes(value, guards, bound, out);
            for arm in arms {
                spec_indexes(&arm.result, &arm_guards(guards, arm), bound, out);
            }
        }
        Expr::Field(..)
        | Expr::Is(..)
        | Expr::Bool(_)
        | Expr::Int(..)
        | Expr::Text(..)
        | Expr::Name(..)
        | Expr::Variant(..)
        | Expr::Next(..) => {}
    }
    if matches!(expr, Expr::Index(..)) {
        out.push(Operation {
            expr,
            guards: guards.to_vec(),
            bound: bound.to_vec(),
        });
    }
}
/// The names of the indices `bound`'s quantifiers range over, outermost
/// first, and `claim` for some value of each: every index in range, and an
/// element's name standing for the element at its index.
pub(super) fn for_some(bound: &[&Expr], claim: String) -> (Vec<String>, String) {
    let mut names = Vec::new();
    let mut out = claim;
    for quantified in bound.iter().rev() {
        let Expr::Quantified(_, name, range, ..) = quantified else {
            continue;
        };
        let index = index_name(name, range);
        out = format!(
            "(and {} {})",
            in_bounds(&term(range.list()), &index),
            at_index(name, range, &out)
        );
        names.insert(0, index);
    }
    (names, out)
}
/// `claim` for every index of `bound`'s quantifiers in range.
pub(super) fn for_every(bound: &[&Expr], claim: String) -> String {
    let mut out = claim;
    for quantified in bound.iter().rev() {
        let Expr::Quantified(_, name, range, ..) = quantified else {
            continue;
        };
        let index = index_name(name, range);
        out = format!(
            "(forall (({index} Int)) (=> {} {}))",
            in_bounds(&term(range.list()), &index),
            at_index(name, range, &out)
        );
    }
    out
}
/// That where `op` runs, an arithmetic operation stays within its type; `None`
/// for any other operation.
pub(super) fn range_assumption(op: &Operation) -> Option<String> {
    let (Expr::Arith(.., site) | Expr::Neg(_, site)) = op.expr else {
        return None;
    };
    let width = site.width?;
    let guards = all("and", op.guards.iter().map(guard_term).collect(), "true");
    let holds = format!("(=> {guards} {})", in_range(&term(op.expr), width));
    Some(for_every(&op.bound, holds))
}

/// That where `op` runs, its callee's `requires` holds; `None` for an
/// operation that isn't a call to a function with a `requires`.
pub(super) fn assumption(program: &Program, op: &Operation) -> Option<String> {
    if !matches!(op.expr, Expr::Call(..)) || !has_requires(program, op.expr) {
        return None;
    }
    let guards = all("and", op.guards.iter().map(guard_term).collect(), "true");
    let holds = format!("(=> {guards} {})", requires_of(program, op.expr));
    Some(for_every(&op.bound, holds))
}
/// Every operation in `function`'s body, in the order they run; a decision
/// table's rows run in order, each only when no earlier row's condition holds.
pub(super) fn body_operations(function: &Function) -> Vec<Operation<'_>> {
    let mut ops = Vec::new();
    match &function.body {
        Body::Expr(body) => operations(body, &[], &[], &mut ops),
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
                    operations(c, &reaches, &[], &mut ops);
                }
                reaches.push(Guard {
                    condition: row.condition.as_ref(),
                    holds: true,
                });
                operations(&row.result, &reaches, &[], &mut ops);
            }
        }
    }
    ops
}
/// Every call in `function`'s body, in the order they run.
pub(super) fn calls_in<'a>(function: &'a Function, out: &mut Vec<Operation<'a>>) {
    out.extend(
        body_operations(function)
            .into_iter()
            .filter(|op| matches!(op.expr, Expr::Call(..))),
    );
}
