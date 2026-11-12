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
//!
//! A call is known to a script only through the callee's contract: where its
//! `requires` holds of the arguments, the result meets its `ensures`. Each
//! call that runs carries the obligation that the callee's `requires` holds.
//! An obligation assumes the `requires` of every call that runs before the
//! operation it is about, and a claim about the body's value assumes them
//! all, since each is an obligation of its own.

use crate::syntax::{Body, ClauseKind, Expr, Function, Program, Width};

mod scripts;
mod smt;

pub(crate) use scripts::{mutant_scripts, open_script};
pub(crate) use smt::{all, datatypes, ending, in_range, sort, term};
use smt::{call_facts, declarations, has_requires, prelude, requires_of, value};

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
    /// Every function the program declares, for running calls.
    pub functions: &'a [Function],
    pub clause: String,
    pub line: usize,
    /// The names a solver's model gives values to: the parameters, then a
    /// constant for each call's value.
    pub params: Vec<String>,
    /// Each call's constant, with the call.
    pub calls: Vec<(String, &'a Expr)>,
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
    /// Where the guards hold, the callee's `requires` holds of `call`'s
    /// arguments.
    Requires {
        call: &'a Expr,
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
/// One arithmetic operation or call in code, with the conditions under which
/// it runs.
struct Operation<'a> {
    expr: &'a Expr,
    guards: Vec<Guard<'a>>,
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
fn assumption(program: &Program, op: &Operation) -> Option<String> {
    if !matches!(op.expr, Expr::Call(..)) || !has_requires(program, op.expr) {
        return None;
    }
    let guards = all("and", op.guards.iter().map(guard_term).collect(), "true");
    Some(format!("(=> {guards} {})", requires_of(program, op.expr)))
}
/// Every operation in `function`'s body, in the order they run; a decision
/// table's rows run in order, each only when no earlier row's condition holds.
fn body_operations(function: &Function) -> Vec<Operation<'_>> {
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
fn obligations_of<'a>(
    function: &'a Function,
    program: &'a Program,
    datatypes: &str,
) -> Vec<Obligation<'a>> {
    let ops = body_operations(function);
    let mut calls = Vec::new();
    calls_in(function, &mut calls);
    let (facts, constants) = call_facts(program, &calls);
    let mut callees = Vec::new();
    for call in &calls {
        crate::check::callees(call, &mut callees);
    }
    let prelude = format!(
        "{}{}{facts}",
        prelude(function, datatypes),
        declarations(program, &callees)
    );
    let mut params: Vec<String> = function
        .params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    params.extend(constants.iter().cloned());
    let named_calls: Vec<(String, &Expr)> = {
        let (mut out, mut seen) = (Vec::new(), Vec::new());
        for call in &calls {
            let t = term(call);
            if !seen.contains(&t) {
                seen.push(t);
                out.push(*call);
            }
        }
        constants.iter().cloned().zip(out).collect()
    };
    let end = ending(&params);
    let assumptions: Vec<Option<String>> = ops.iter().map(|op| assumption(program, op)).collect();
    let before =
        |index: usize| -> Vec<String> { assumptions[..index].iter().flatten().cloned().collect() };
    let every = before(assumptions.len());
    let obligation = |clause: &str,
                      line: usize,
                      polarity: Polarity,
                      claim: Claim<'a>,
                      declare: &str,
                      assumed: &[String],
                      assertion: String| {
        let mut conjuncts = assumed.to_vec();
        conjuncts.push(assertion);
        Obligation {
            function,
            functions: &program.functions,
            clause: clause.to_string(),
            line,
            params: params.clone(),
            calls: named_calls.clone(),
            polarity,
            claim,
            script: format!(
                "{prelude}{declare}(assert {})\n{end}",
                all("and", conjuncts, "true")
            ),
        }
    };
    let result = format!("(declare-const result {})\n", sort(&function.ret));

    let (body, covered) = value(function);
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
            Claim::Ensures(&clause.expr),
            &result,
            &every,
            assertion,
        ));
    }
    if let (Body::Decide(rows, line), Some(covered)) = (&function.body, covered) {
        out.push(obligation(
            "every input reaches a row",
            *line,
            Polarity::Universal,
            Claim::Covered,
            "",
            &every,
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
            reaches.push(Guard {
                condition: row.condition.as_ref(),
                holds: true,
            });
            out.push(obligation(
                &row.text,
                row.line,
                Polarity::Existential,
                Claim::Reaches(index),
                "",
                &every,
                all("and", reaches.iter().map(guard_term).collect(), "true"),
            ));
        }
    }
    for (index, op) in ops.into_iter().enumerate() {
        let mut assertion: Vec<String> = op.guards.iter().map(guard_term).collect();
        let (clause, line, claim) = match op.expr {
            Expr::Arith(.., site) | Expr::Neg(_, site) => {
                let Some(width) = site.width else {
                    // `check` sets every width before obligations are generated.
                    unreachable!("arithmetic without a width: `{}`", site.text)
                };
                assertion.push(format!("(not {})", in_range(&term(op.expr), width)));
                (
                    format!("`{}` stays within `{}`", site.text, width.name()),
                    site.line,
                    Claim::InRange {
                        expr: op.expr,
                        width,
                        guards: op.guards,
                    },
                )
            }
            Expr::Call(name, _, site) if has_requires(program, op.expr) => {
                assertion.push(format!("(not {})", requires_of(program, op.expr)));
                // The call as written starts with its callee's source name;
                // `name` may be normalized.
                let callee = site.text.split('(').next().unwrap_or(name).trim();
                (
                    format!("`{}` meets `{callee}`'s requires", site.text),
                    site.line,
                    Claim::Requires {
                        call: op.expr,
                        guards: op.guards,
                    },
                )
            }
            _ => continue,
        };
        out.push(obligation(
            &clause,
            line,
            Polarity::Universal,
            claim,
            "",
            &before(index),
            all("and", assertion, "true"),
        ));
    }
    // Stable, so obligations on one line keep the order above.
    out.sort_by_key(|o| o.line);
    out
}
/// The obligations of every function, in source order: one per `ensures`;
/// for a decision table, that every input reaches a row and that each row
/// can be the first to hold; that each arithmetic operation in code stays
/// in its type's range; and that each call meets its callee's `requires`.
#[cfg(test)]
pub(crate) fn obligations(program: &Program) -> Vec<Obligation<'_>> {
    let datatypes = datatypes(program);
    program
        .functions
        .iter()
        .flat_map(|function| obligations_of(function, program, &datatypes))
        .collect()
}
/// The obligations of a unit's function, the first in its program; the
/// others are its callees, known only through their contracts.
pub(crate) fn unit_obligations(program: &Program) -> Vec<Obligation<'_>> {
    let datatypes = datatypes(program);
    program
        .functions
        .first()
        .map(|function| obligations_of(function, program, &datatypes))
        .unwrap_or_default()
}
