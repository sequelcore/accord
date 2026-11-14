//! Writing SMT-LIB: terms, sorts and ranges, a function's prelude, and the
//! ending every script shares.

use super::calls::apply;
use super::lists::{MAX_LENGTH, contains, quantified};
use crate::syntax::{
    Arith, Body, ClauseKind, Enum, Expr, Function, Op, Owner, Program, Role, Row, Type, Width,
};

pub(crate) fn sort(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int(_) => "Int".to_string(),
        Type::Text => "String".to_string(),
        Type::List(element) => format!("(Seq {})", sort(element)),
        Type::Enum(name) | Type::Record(name) => name.clone(),
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
        Expr::Construct(owner, variant, args, _) => apply(
            &format!("{owner}.{variant}"),
            args.iter().map(|(_, a)| term(a)).collect(),
        ),
        Expr::Record(name, fields, _) => {
            apply(name, fields.iter().map(|(_, value)| term(value)).collect())
        }
        Expr::Field(value, owner, field, _) => match owner {
            Some(Owner::Variant(e, variant)) => format!("({e}.{variant}.{field} {})", term(value)),
            Some(Owner::Record(name)) => format!("({name}.{field} {})", term(value)),
            None => unreachable!("`check` resolves every field: `{field}`"),
        },
        // The arms' tests cover every variant, so the last arm needs none.
        Expr::Match(_, arms, _) => {
            let Some((last, earlier)) = arms.split_last() else {
                return String::new();
            };
            earlier.iter().rev().fold(term(&last.result), |rest, arm| {
                let test = arm.test.as_ref().map_or("true".to_string(), term);
                format!("(ite {test} {} {rest})", term(&arm.result))
            })
        }
        Expr::Is(value, e, variant) => format!("((_ is {e}.{variant}) {})", term(value)),
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
/// That `term`, a value of `ty`, is one a program can hold: each integer in
/// it within its width and each list within its length; `None` when every
/// value of `ty` is.
pub(crate) fn fits(term: &str, ty: &Type, program: &Program) -> Option<String> {
    let conjunction = |parts: Vec<String>| (!parts.is_empty()).then(|| all("and", parts, "true"));
    match ty {
        Type::Int(width) => Some(in_range(term, *width)),
        Type::List(_) => Some(format!("(<= (seq.len {term}) {MAX_LENGTH})")),
        Type::Record(name) => {
            let record = program.records.iter().find(|r| r.name == *name)?;
            conjunction(
                record
                    .fields
                    .iter()
                    .filter_map(|(field, ty)| {
                        fits(&format!("({name}.{field} {term})"), ty, program)
                    })
                    .collect(),
            )
        }
        Type::Enum(name) => {
            let e = program.enums.iter().find(|e| e.name == *name)?;
            conjunction(
                e.variants
                    .iter()
                    .zip(&e.fields)
                    .filter_map(|(variant, fields)| {
                        let each = conjunction(
                            fields
                                .iter()
                                .filter_map(|(field, ty)| {
                                    fits(&format!("({name}.{variant}.{field} {term})"), ty, program)
                                })
                                .collect(),
                        )?;
                        Some(format!("(=> ((_ is {name}.{variant}) {term}) {each})"))
                    })
                    .collect(),
            )
        }
        Type::Bool | Type::Text => None,
    }
}
/// What every script about `function` starts with: the types it uses, its
/// parameters within their ranges, and its `requires` assumed.
pub(crate) fn prelude(function: &Function, program: &Program, datatypes: &str) -> String {
    let mut prelude = format!("(set-option :produce-models true)\n(set-logic ALL)\n{datatypes}");
    for (name, ty) in &function.params {
        prelude += &format!("(declare-const {name} {})\n", sort(ty));
        if let Some(fact) = fits(name, ty, program) {
            prelude += &format!("(assert {fact})\n");
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
/// An enum's constructors: each variant, with a selector for each field.
fn constructors(e: &Enum) -> String {
    let constructors: Vec<String> = e
        .variants
        .iter()
        .zip(&e.fields)
        .map(|(v, fields)| {
            let selectors: String = fields
                .iter()
                .map(|(field, ty)| format!(" ({}.{v}.{field} {})", e.name, sort(ty)))
                .collect();
            format!("({}.{v}{selectors})", e.name)
        })
        .collect();
    format!("({})", constructors.join(" "))
}
/// The SMT-LIB declarations of a program's enums and records. Enums without
/// fields come first, each on its own; the rest are declared together, so
/// each may hold the others whatever their order.
pub(crate) fn datatypes(program: &Program) -> String {
    let (plain, structured): (Vec<&Enum>, Vec<&Enum>) =
        program.enums.iter().partition(|e| !e.has_fields());
    let mut out: String = plain
        .iter()
        .map(|e| format!("(declare-datatype {} {})\n", e.name, constructors(e)))
        .collect();
    let mut names: Vec<&str> = structured.iter().map(|e| e.name.as_str()).collect();
    let mut bodies: Vec<String> = structured.iter().map(|e| constructors(e)).collect();
    for record in &program.records {
        names.push(&record.name);
        let selectors: String = record
            .fields
            .iter()
            .map(|(field, ty)| format!(" ({}.{field} {})", record.name, sort(ty)))
            .collect();
        bodies.push(format!("(({}{selectors}))", record.name));
    }
    if !names.is_empty() {
        let sorts: Vec<String> = names.iter().map(|name| format!("({name} 0)")).collect();
        out += &format!(
            "(declare-datatypes ({}) ({}))\n",
            sorts.join(" "),
            bodies.join(" ")
        );
    }
    out
}
