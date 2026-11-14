//! The mistakes planted in a body, and how each is described.

use crate::identity::Unit;
use crate::syntax::{Body, Expr, Function, Op, Owner, Program};

/// A planted change, before it's written in source terms.
pub(super) enum Change {
    /// An operator replaced, with the line of its left operand.
    Op(usize, Op, Op),
    /// A literal changed.
    Literal(usize, i128, i128),
    /// A condition negated.
    Negated(usize),
    /// A membership test negated, with the line of its element.
    NotIn(usize),
    /// Two adjacent rows swapped, by line.
    Swapped(usize, usize),
    /// A row's result replaced.
    Result(usize, String, String),
    /// A row removed.
    Dropped(usize),
}

fn op_text(op: Op) -> &'static str {
    match op {
        Op::And => "and",
        Op::Or => "or",
        Op::Implies => "implies",
        Op::Eq => "==",
        Op::Ne => "!=",
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Gt => ">",
        Op::Ge => ">=",
        Op::In => "in",
    }
}

/// The operator a mutant puts in place of `op`: a boundary moved, an
/// equality flipped, a connective swapped.
fn swap(op: Op) -> Option<Op> {
    Some(match op {
        Op::Lt => Op::Le,
        Op::Le => Op::Lt,
        Op::Gt => Op::Ge,
        Op::Ge => Op::Gt,
        Op::Eq => Op::Ne,
        Op::Ne => Op::Eq,
        Op::And => Op::Or,
        Op::Or => Op::And,
        Op::Implies | Op::In => return None,
    })
}

/// Whether the change being counted down to is this one; counts it down
/// otherwise.
fn at(n: &mut usize) -> bool {
    let here = *n == 0;
    *n = n.wrapping_sub(1);
    here
}

/// Applies the `n`th change in `expr`, counting down `n`; `None` while the
/// change lies further on.
fn mutate_expr(expr: &mut Expr, n: &mut usize) -> Option<Change> {
    if let Expr::Binary(Op::In, element, _) = expr
        && at(n)
    {
        let line = element.line();
        *expr = Expr::Not(Box::new(expr.clone()));
        return Some(Change::NotIn(line));
    }
    match expr {
        Expr::Binary(op, left, right) => {
            if let Some(to) = swap(*op)
                && at(n)
            {
                let change = Change::Op(left.line(), *op, to);
                *op = to;
                return Some(change);
            }
            mutate_expr(left, n).or_else(|| mutate_expr(right, n))
        }
        Expr::Int(value, line) => {
            for to in [*value + 1, *value - 1] {
                if to >= 0 && at(n) {
                    let change = Change::Literal(*line, *value, to);
                    *value = to;
                    return Some(change);
                }
            }
            None
        }
        Expr::If(condition, then, otherwise) => {
            if at(n) {
                let line = condition.line();
                **condition = Expr::Not(condition.clone());
                return Some(Change::Negated(line));
            }
            mutate_expr(condition, n)
                .or_else(|| mutate_expr(then, n))
                .or_else(|| mutate_expr(otherwise, n))
        }
        Expr::Not(inner) | Expr::Neg(inner, _) => mutate_expr(inner, n),
        Expr::Arith(_, left, right, _) => mutate_expr(left, n).or_else(|| mutate_expr(right, n)),
        Expr::List(items, ..) => items.iter_mut().find_map(|item| mutate_expr(item, n)),
        Expr::Call(_, args, _) => args.iter_mut().find_map(|(_, arg)| mutate_expr(arg, n)),
        Expr::Quantified(_, _, range, condition, _) => {
            mutate_expr(range.list_mut(), n).or_else(|| mutate_expr(condition, n))
        }
        Expr::Length(list, _) => mutate_expr(list, n),
        Expr::Next(fields, _) => fields
            .iter_mut()
            .find_map(|(_, value)| mutate_expr(value, n)),
        Expr::Index(list, index, _) => mutate_expr(list, n).or_else(|| mutate_expr(index, n)),
        Expr::Construct(_, _, args, _) => args.iter_mut().find_map(|(_, arg)| mutate_expr(arg, n)),
        Expr::Record(_, fields, _) => fields
            .iter_mut()
            .find_map(|(_, value)| mutate_expr(value, n)),
        Expr::Field(value, Some(Owner::Record(_)), ..) => mutate_expr(value, n),
        // The value matched is read again by every arm's test and fields, so
        // only the arms' results change.
        Expr::Match(_, arms, _) => arms
            .iter_mut()
            .find_map(|arm| mutate_expr(&mut arm.result, n)),
        Expr::Field(..)
        | Expr::Is(..)
        | Expr::Bool(_)
        | Expr::Text(..)
        | Expr::Name(..)
        | Expr::Variant(..) => None,
    }
}

/// Applies the `n`th change to `function`'s body.
pub(super) fn mutate(function: &mut Function, program: &Program, mut n: usize) -> Option<Change> {
    match &mut function.body {
        Body::Expr(body) => mutate_expr(body, &mut n),
        Body::Decide(rows, _) => {
            // Adjacent rows swapped; `otherwise` stays last.
            for i in 0..rows.len().saturating_sub(1) {
                if rows[i + 1].condition.is_some() && at(&mut n) {
                    let lines = (rows[i].line, rows[i + 1].line);
                    rows.swap(i, i + 1);
                    return Some(Change::Swapped(lines.0, lines.1));
                }
            }
            // A row's result replaced by another value of its type.
            for row in rows.iter_mut() {
                let others: Vec<Expr> = match &row.result {
                    Expr::Bool(b) => vec![Expr::Bool(!b)],
                    Expr::Variant(owner, variant, line) => program
                        .enums
                        .iter()
                        .find(|e| e.name == *owner)
                        .map(|e| {
                            e.variants
                                .iter()
                                .filter(|v| *v != variant && e.fields_of(v).is_empty())
                                .map(|v| Expr::Variant(owner.clone(), v.clone(), *line))
                                .collect()
                        })
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                for other in others {
                    if at(&mut n) {
                        let change = Change::Result(row.line, text(&row.result), text(&other));
                        row.result = other;
                        return Some(change);
                    }
                }
            }
            // A row removed, when `otherwise` keeps the table complete.
            if rows.last().is_some_and(|r| r.condition.is_none()) {
                for i in 0..rows.len() - 1 {
                    if at(&mut n) {
                        let line = rows[i].line;
                        rows.remove(i);
                        return Some(Change::Dropped(line));
                    }
                }
            }
            // A condition negated, then changes inside conditions and results.
            for row in rows.iter_mut() {
                if let Some(condition) = &mut row.condition {
                    if at(&mut n) {
                        *condition = Expr::Not(Box::new(condition.clone()));
                        return Some(Change::Negated(row.line));
                    }
                    if let Some(change) = mutate_expr(condition, &mut n) {
                        return Some(change);
                    }
                }
                if let Some(change) = mutate_expr(&mut row.result, &mut n) {
                    return Some(change);
                }
            }
            None
        }
    }
}

/// A row result's normalized text, for its description.
fn text(expr: &Expr) -> String {
    match expr {
        Expr::Bool(b) => b.to_string(),
        Expr::Variant(owner, variant, _) => format!("{owner}.{variant}"),
        _ => String::new(),
    }
}

pub(super) fn describe(change: &Change, unit: &Unit) -> String {
    match change {
        Change::Op(line, from, to) => format!(
            "line {line}: `{}` became `{}`",
            op_text(*from),
            op_text(*to)
        ),
        Change::Literal(line, from, to) => format!("line {line}: `{from}` became `{to}`"),
        Change::Negated(line) => format!("line {line}: the condition negated"),
        Change::NotIn(line) => format!("line {line}: `in` became `not in`"),
        Change::Swapped(first, second) => format!("the rows on lines {first} and {second} swapped"),
        Change::Result(line, from, to) => format!(
            "line {line}: the row gives `{}` instead of `{}`",
            short(&unit.source_value(to)),
            short(&unit.source_value(from))
        ),
        Change::Dropped(line) => format!("the row on line {line} removed"),
    }
}

/// A variant without its enum's name, as a table row writes it.
fn short(value: &str) -> &str {
    value.split_once('.').map_or(value, |(_, v)| v)
}
