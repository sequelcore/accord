//! Turns a checked program into SMT-LIB scripts, one per obligation.
//!
//! A universal obligation must hold for every admitted input: its script
//! asserts that it fails, so `unsat` proves it. An existential obligation
//! needs some admitted input, such as one that reaches a decision table's
//! row: its script asserts that the input exists, so `sat` proves it.
//!
//! Integers are mathematical in every script. A parameter of a fixed width is
//! assumed to be in its range, and each arithmetic operation in code that
//! runs carries the obligation that its result stays in range, so the
//! mathematical value is the one the program computes.

use crate::syntax::{Body, ClauseKind, Expr, Function, Program, Site, Width};

mod scripts;
mod smt;

pub(crate) use scripts::{mutant_scripts, open_script};
pub(crate) use smt::{all, datatypes, ending, in_range, sort, term};
use smt::{prelude, value};

/// Whether an obligation is about every input or about some input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Polarity {
    Universal,
    Existential,
}
/// One logical statement that must hold, written as an SMT-LIB script, with
/// what it claims in source terms so a solver's model can be replayed.
pub(crate) struct Obligation<'a> {
    pub function: &'a Function,
    pub clause: String,
    pub line: usize,
    pub params: Vec<String>,
    pub polarity: Polarity,
    pub claim: Claim<'a>,
    pub script: String,
}
/// What an obligation states, for admitted inputs (every `requires` holds).
pub(crate) enum Claim<'a> {
    /// The `ensures` holds of the body's value.
    Ensures(&'a Expr),
    /// Some row of the decision table matches.
    Covered,
    /// The row at this index is the first that matches.
    Reaches(usize),
    /// Where the guards hold, `expr` stays within `width`.
    InRange {
        expr: &'a Expr,
        width: Width,
        guards: Vec<Guard<'a>>,
    },
}
/// A condition that must have a given value for code to run; `None` is a
/// row's `otherwise`, which always holds.
#[derive(Clone, Copy)]
pub(crate) struct Guard<'a> {
    pub condition: Option<&'a Expr>,
    pub holds: bool,
}
fn guard_term(guard: &Guard) -> String {
    let c = guard.condition.map_or("true".to_string(), term);
    if guard.holds { c } else { format!("(not {c})") }
}
/// One arithmetic operation in code, with the conditions under which it runs.
struct Overflow<'a> {
    expr: &'a Expr,
    site: &'a Site,
    guards: Vec<Guard<'a>>,
}
/// Every arithmetic operation in `expr`, inner ones first. Only the branch
/// of an `if` that runs is guarded by its condition; both operands of `and`
/// and `or` are checked, whatever the other's value.
fn overflows<'a>(expr: &'a Expr, guards: &[Guard<'a>], out: &mut Vec<Overflow<'a>>) {
    match expr {
        Expr::Arith(_, left, right, site) => {
            overflows(left, guards, out);
            overflows(right, guards, out);
            out.push(Overflow {
                expr,
                site,
                guards: guards.to_vec(),
            });
        }
        Expr::Neg(inner, site) => {
            overflows(inner, guards, out);
            out.push(Overflow {
                expr,
                site,
                guards: guards.to_vec(),
            });
        }
        Expr::Not(inner) => overflows(inner, guards, out),
        Expr::Binary(_, left, right) => {
            overflows(left, guards, out);
            overflows(right, guards, out);
        }
        Expr::If(condition, then, otherwise) => {
            overflows(condition, guards, out);
            let guard = |holds| Guard {
                condition: Some(condition),
                holds,
            };
            overflows(then, &[guards, &[guard(true)]].concat(), out);
            overflows(otherwise, &[guards, &[guard(false)]].concat(), out);
        }
        Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
}
fn obligations_of<'a>(function: &'a Function, datatypes: &str) -> Vec<Obligation<'a>> {
    let prelude = prelude(function, datatypes);
    let params: Vec<String> = function
        .params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let end = ending(&params);
    let obligation = |clause: &str,
                      line: usize,
                      polarity: Polarity,
                      claim: Claim<'a>,
                      declare: &str,
                      assertion: String| Obligation {
        function,
        clause: clause.to_string(),
        line,
        params: params.clone(),
        polarity,
        claim,
        script: format!("{prelude}{declare}(assert {assertion})\n{end}"),
    };
    let result = format!("(declare-const result {})\n", sort(&function.ret));

    let (body, covered) = value(function);
    let rows = match &function.body {
        Body::Expr(_) => None,
        Body::Decide(rows, line) => Some((rows, *line)),
    };
    let mut out = Vec::new();
    let mut ops = Vec::new();
    if let Body::Expr(body) = &function.body {
        overflows(body, &[], &mut ops);
    }
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
            Claim::Ensures(&clause.expr),
            &result,
            assertion,
        ));
    }
    if let (Some((rows, line)), Some(covered)) = (rows, covered) {
        out.push(obligation(
            "every input reaches a row",
            line,
            Polarity::Universal,
            Claim::Covered,
            "",
            format!("(not {covered})"),
        ));
        for (index, row) in rows.iter().enumerate() {
            let mut reaches: Vec<Guard> = rows[..index]
                .iter()
                .map(|earlier| Guard {
                    condition: earlier.condition.as_ref(),
                    holds: false,
                })
                .collect();
            if let Some(c) = &row.condition {
                overflows(c, &reaches, &mut ops);
            }
            reaches.push(Guard {
                condition: row.condition.as_ref(),
                holds: true,
            });
            overflows(&row.result, &reaches, &mut ops);
            out.push(obligation(
                &row.text,
                row.line,
                Polarity::Existential,
                Claim::Reaches(index),
                "",
                all("and", reaches.iter().map(guard_term).collect(), "true"),
            ));
        }
    }
    for op in ops {
        let Some(width) = op.site.width else {
            // `check` sets every width before obligations are generated.
            unreachable!("arithmetic without a width: `{}`", op.site.text)
        };
        let mut assertion: Vec<String> = op.guards.iter().map(guard_term).collect();
        assertion.push(format!("(not {})", in_range(&term(op.expr), width)));
        out.push(obligation(
            &format!("`{}` stays within `{}`", op.site.text, width.name()),
            op.site.line,
            Polarity::Universal,
            Claim::InRange {
                expr: op.expr,
                width,
                guards: op.guards,
            },
            "",
            all("and", assertion, "true"),
        ));
    }
    // Stable, so obligations on one line keep the order above.
    out.sort_by_key(|o| o.line);
    out
}
/// The obligations of every function, in source order: one per `ensures`;
/// for a decision table, that every input reaches a row and that each row
/// can be the first to hold; and that each arithmetic operation in code stays
/// in its type's range.
pub(crate) fn obligations(program: &Program) -> Vec<Obligation<'_>> {
    let datatypes = datatypes(program);
    program
        .functions
        .iter()
        .flat_map(|function| obligations_of(function, &datatypes))
        .collect()
}
