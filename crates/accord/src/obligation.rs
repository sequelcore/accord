//! Turns a checked program into SMT-LIB scripts, one per obligation.
//!
//! A universal obligation must hold for every admitted input: its script
//! asserts that it fails, so `unsat` proves it. An existential obligation
//! needs some admitted input, such as one that reaches a decision table's
//! row: its script asserts that the input exists, so `sat` proves it.

use crate::syntax::{Body, ClauseKind, Expr, Function, Op, Program, Row, Type};

/// Whether an obligation is about every input or about some input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Polarity {
    Universal,
    Existential,
}

/// One logical statement that must hold, written as an SMT-LIB script.
pub(crate) struct Obligation {
    pub function: String,
    pub clause: String,
    pub line: usize,
    pub params: Vec<String>,
    pub polarity: Polarity,
    pub script: String,
}

fn sort(ty: &Type) -> &str {
    match ty {
        Type::Bool => "Bool",
        Type::Enum(name) => name,
    }
}

fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
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
            }
        }
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

fn condition(row: &Row) -> String {
    row.condition.as_ref().map_or("true".to_string(), term)
}

/// `and` or `or` over any number of terms; SMT-LIB needs two or more.
fn all(op: &str, terms: Vec<String>, empty: &str) -> String {
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

fn obligations_of(function: &Function, datatypes: &str) -> Vec<Obligation> {
    let mut prelude = format!("(set-option :produce-models true)\n(set-logic ALL)\n{datatypes}");
    for (name, ty) in &function.params {
        prelude += &format!("(declare-const {name} {})\n", sort(ty));
    }
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        prelude += &format!("(assert {})\n", term(&clause.expr));
    }
    let params: Vec<String> = function
        .params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let values = if params.is_empty() {
        String::new()
    } else {
        format!("(get-value ({}))\n", params.join(" "))
    };
    let obligation =
        |clause: &str, line: usize, polarity: Polarity, declare: &str, assertion: String| {
            Obligation {
                function: function.name.clone(),
                clause: clause.to_string(),
                line,
                params: params.clone(),
                polarity,
                script: format!("{prelude}{declare}(assert {assertion})\n(check-sat)\n{values}"),
            }
        };
    let result = format!("(declare-const result {})\n", sort(&function.ret));

    let (body, rows) = match &function.body {
        Body::Expr(body) => (term(body), None),
        Body::Decide(rows, line) => (decide_term(rows), Some((rows, *line))),
    };
    let covered = rows.map(|(rows, _)| all("or", rows.iter().map(condition).collect(), "false"));
    let mut out = Vec::new();
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures)
    {
        let mut assertion = format!("(and (= result {body}) (not {}))", term(&clause.expr));
        if let Some(covered) = &covered {
            assertion = format!("(and {covered} {assertion})");
        }
        out.push(obligation(
            &clause.text,
            clause.line,
            Polarity::Universal,
            &result,
            assertion,
        ));
    }
    if let (Some((rows, line)), Some(covered)) = (rows, covered) {
        out.push(obligation(
            "every input reaches a row",
            line,
            Polarity::Universal,
            "",
            format!("(not {covered})"),
        ));
        for (index, row) in rows.iter().enumerate() {
            let mut reaches: Vec<String> = rows[..index]
                .iter()
                .map(|earlier| format!("(not {})", condition(earlier)))
                .collect();
            reaches.push(condition(row));
            out.push(obligation(
                &row.text,
                row.line,
                Polarity::Existential,
                "",
                all("and", reaches, "true"),
            ));
        }
    }
    out
}

/// The obligations of every function, in source order: one per `ensures`,
/// then, for a decision table, that every input reaches a row and that each
/// row can be the first to hold.
pub(crate) fn obligations(program: &Program) -> Vec<Obligation> {
    let datatypes: String = program
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
        .collect();
    program
        .functions
        .iter()
        .flat_map(|function| obligations_of(function, &datatypes))
        .collect()
}
