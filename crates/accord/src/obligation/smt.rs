//! Writing SMT-LIB: terms, sorts and ranges, a function's prelude, and the
//! ending every script shares.

use super::calls::apply;
use super::lists::{MAX_LENGTH, contains, quantified};
use crate::syntax::{Arith, Body, ClauseKind, Expr, Function, Op, Program, Role, Row, Type, Width};

pub(crate) fn sort(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int(_) => "Int".to_string(),
        Type::Text => "String".to_string(),
        Type::List(element) => format!("(Seq {})", sort(element)),
        Type::Enum(name) => name.clone(),
    }
}
pub(crate) fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::Int(value, _) => number(*value),
        Expr::Text(text, _) => string(text),
        Expr::List(items, element, _) => {
            let units: Vec<String> = items
                .iter()
                .map(|i| format!("(seq.unit {})", term(i)))
                .collect();
            match units.len() {
                0 => format!(
                    "(as seq.empty (Seq {}))",
                    element.as_ref().map_or("Bool".to_string(), sort)
                ),
                1 => units.concat(),
                _ => format!("(seq.++ {})", units.join(" ")),
            }
        }
        Expr::Name(name, _) => name.clone(),
        Expr::Variant(owner, name, _) => format!("{owner}.{name}"),
        Expr::Not(inner) => format!("(not {})", term(inner)),
        Expr::Binary(op, left, right) => {
            let (l, r) = (term(left), term(right));
            match op {
                Op::And => format!("(and {l} {r})"),
                Op::Or => format!("(or {l} {r})"),
                Op::Implies => format!("(=> {l} {r})"),
                Op::Eq => format!("(= {l} {r})"),
                Op::Ne => format!("(not (= {l} {r}))"),
                Op::Lt => format!("(< {l} {r})"),
                Op::Le => format!("(<= {l} {r})"),
                Op::Gt => format!("(> {l} {r})"),
                Op::Ge => format!("(>= {l} {r})"),
                Op::In => contains(right, &l),
            }
        }
        Expr::Arith(op, left, right, _) => {
            let symbol = match op {
                Arith::Add => "+",
                Arith::Sub => "-",
                Arith::Mul => "*",
            };
            format!("({symbol} {} {})", term(left), term(right))
        }
        Expr::Neg(inner, _) => format!("(- {})", term(inner)),
        Expr::If(condition, then, otherwise) => {
            format!(
                "(ite {} {} {})",
                term(condition),
                term(then),
                term(otherwise)
            )
        }
        Expr::Call(name, args, _) => apply(name, args.iter().map(|(_, a)| term(a)).collect()),
        Expr::Quantified(quantifier, name, range, condition, _) => {
            quantified(*quantifier, name, range, condition)
        }
        Expr::Length(list, _) => format!("(seq.len {})", term(list)),
        Expr::Index(list, index, _) => format!("(seq.nth {} {})", term(list), term(index)),
        // Identity reads a `next` as one term; a script reads it field by
        // field (`meets_invariants`).
        Expr::Next(values, _) => format!(
            "(next {})",
            values
                .iter()
                .map(|(_, value)| term(value))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    }
}
/// A text as an SMT-LIB string literal. SMT-LIB reads `\u{...}` in a
/// literal as an escape, so a backslash, and every character outside
/// printable ASCII, is written as one; the text means the same to the
/// solvers as to the program.
fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\"\""),
            ' '..='~' if c != '\\' => out.push(c),
            _ => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
        }
    }
    out.push('"');
    out
}

fn number(value: i128) -> String {
    if value < 0 {
        format!("(- {})", value.unsigned_abs())
    } else {
        value.to_string()
    }
}
/// That `term` is a value of `width`.
pub(crate) fn in_range(term: &str, width: Width) -> String {
    format!(
        "(and (<= {} {term}) (<= {term} {}))",
        number(width.min()),
        number(width.max())
    )
}
fn condition(row: &Row) -> String {
    row.condition.as_ref().map_or("true".to_string(), term)
}
/// `and` or `or` over any number of terms; SMT-LIB needs two or more.
pub(crate) fn all(op: &str, terms: Vec<String>, empty: &str) -> String {
    match terms.len() {
        0 => empty.to_string(),
        1 => terms.concat(),
        _ => format!("({op} {})", terms.join(" ")),
    }
}
/// The value of a decision table: the first row whose condition holds. Past
/// the last row the value is the last row's; coverage is its own obligation.
fn decide_term(rows: &[Row]) -> String {
    let Some((last, earlier)) = rows.split_last() else {
        return String::new();
    };
    earlier.iter().rev().fold(term(&last.result), |rest, row| {
        format!("(ite {} {} {rest})", condition(row), term(&row.result))
    })
}
/// What every script about `function` starts with: the enums it uses, its
/// parameters within their ranges, and its `requires` assumed.
pub(crate) fn prelude(function: &Function, datatypes: &str) -> String {
    let mut prelude = format!("(set-option :produce-models true)\n(set-logic ALL)\n{datatypes}");
    for (name, ty) in &function.params {
        prelude += &format!("(declare-const {name} {})\n", sort(ty));
        match ty {
            Type::Int(width) => prelude += &format!("(assert {})\n", in_range(name, *width)),
            Type::List(_) => {
                prelude += &format!("(assert (<= (seq.len {name}) {MAX_LENGTH}))\n");
            }
            _ => {}
        }
    }
    // An init has no state before it, so no invariant to assume.
    let reads_state = function.machine.is_none_or(|m| m.role != Role::Init);
    for clause in function.clauses.iter().filter(|c| match c.kind {
        ClauseKind::Requires => true,
        ClauseKind::Invariant => reads_state,
        ClauseKind::Ensures | ClauseKind::Open => false,
    }) {
        prelude += &format!("(assert {})\n", term(&clause.expr));
    }
    prelude
}
/// A script's ending: check, then the values of `names`, then why an
/// `unknown` was unknown. Asked last, so its error after `sat` never precedes
/// the values.
pub(crate) fn ending(names: &[String]) -> String {
    let values = if names.is_empty() {
        String::new()
    } else {
        format!("(get-value ({}))\n", names.join(" "))
    };
    format!("(check-sat)\n{values}(get-info :reason-unknown)\n")
}
/// A body's value as a term, and for a decision table the condition that some
/// row matches.
/// An init or event has no value to speak of: its `next` is read field by
/// field.
pub(crate) fn value(function: &Function) -> (String, Option<String>) {
    let transition = function.machine.is_some_and(|m| m.role != Role::Observer);
    match &function.body {
        Body::Expr(_) if transition => (String::new(), None),
        Body::Expr(body) => (term(body), None),
        Body::Decide(rows, _) => (
            if transition {
                String::new()
            } else {
                decide_term(rows)
            },
            Some(all("or", rows.iter().map(condition).collect(), "false")),
        ),
    }
}

/// The terms of the fields of the state an init or event ends in, in order:
/// the field's value in the branch or row that runs.
fn next_terms(function: &Function) -> Vec<String> {
    let count = function.machine.map_or(0, |m| m.state);
    (0..count)
        .map(|field| match &function.body {
            Body::Expr(body) => next_term(body, field),
            Body::Decide(rows, _) => {
                let Some((last, earlier)) = rows.split_last() else {
                    return String::new();
                };
                earlier
                    .iter()
                    .rev()
                    .fold(next_term(&last.result, field), |rest, row| {
                        format!(
                            "(ite {} {} {rest})",
                            condition(row),
                            next_term(&row.result, field)
                        )
                    })
            }
        })
        .collect()
}

fn next_term(expr: &Expr, field: usize) -> String {
    match expr {
        Expr::If(c, then, otherwise) => format!(
            "(ite {} {} {})",
            term(c),
            next_term(then, field),
            next_term(otherwise, field)
        ),
        Expr::Next(values, _) => term(&values[field].1),
        _ => unreachable!("`check` ends every init and event in a `next`"),
    }
}

/// That the state an init or event ends in meets every invariant and helper
/// invariant: they hold of the fields as `next` gives them.
pub(crate) fn meets_invariants(function: &Function) -> String {
    let count = function.machine.map_or(0, |m| m.state);
    let invariants = all(
        "and",
        function
            .clauses
            .iter()
            .filter(|c| c.kind == ClauseKind::Invariant)
            .map(|c| term(&c.expr))
            .collect(),
        "true",
    );
    if count == 0 {
        return invariants;
    }
    let bindings: Vec<String> = function.params[..count]
        .iter()
        .zip(next_terms(function))
        .map(|((field, _), next)| format!("({field} {next})"))
        .collect();
    format!("(let ({}) {invariants})", bindings.join(" "))
}
/// The SMT-LIB declarations of a program's enums.
pub(crate) fn datatypes(program: &Program) -> String {
    program
        .enums
        .iter()
        .map(|e| {
            let constructors: Vec<String> = e
                .variants
                .iter()
                .map(|v| format!("({}.{v})", e.name))
                .collect();
            format!(
                "(declare-datatype {} ({}))\n",
                e.name,
                constructors.join(" ")
            )
        })
        .collect()
}
