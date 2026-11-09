//! Writing SMT-LIB: terms, sorts and ranges, a function's prelude, and the
//! ending every script shares.

use crate::syntax::{Arith, Body, ClauseKind, Expr, Function, Op, Program, Row, Type, Width};

pub(crate) fn sort(ty: &Type) -> &str {
    match ty {
        Type::Bool => "Bool",
        Type::Int(_) => "Int",
        Type::Text => "String",
        Type::Enum(name) => name,
    }
}
pub(crate) fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::Int(value, _) => number(*value),
        Expr::Text(text, _) => string(text),
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
        if let Type::Int(width) = ty {
            prelude += &format!("(assert {})\n", in_range(name, *width));
        }
    }
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
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
pub(crate) fn value(function: &Function) -> (String, Option<String>) {
    match &function.body {
        Body::Expr(body) => (term(body), None),
        Body::Decide(rows, _) => (
            decide_term(rows),
            Some(all("or", rows.iter().map(condition).collect(), "false")),
        ),
    }
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
