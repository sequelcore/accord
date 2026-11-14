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
//!
//! Each index carries the obligation that it is in range, in code and in an
//! `ensures`. An operation inside a quantifier's condition is about every
//! index the quantifier ranges over: its script asks for one where it fails.

use crate::syntax::{Body, ClauseKind, Expr, Function, Program, Role};

mod calls;
mod claims;
mod lists;
mod operations;
mod scripts;
mod smt;

use calls::{call_facts, declarations};
pub(crate) use claims::Claim;
use claims::claim_of;
use lists::list_facts;
pub(crate) use operations::Guard;
use operations::{
    Operation, assumption, body_operations, for_some, guard_term, range_assumption, spec_indexes,
};
pub(crate) use scripts::{mutant_scripts, open_script};
pub(crate) use smt::{all, datatypes, ending, fits, sort, term};
use smt::{meets_invariants, prelude, value};

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
    /// The names a solver's model gives values to: the parameters, a
    /// constant for each call's value, then each index the claim is about.
    pub params: Vec<String>,
    /// Each call's constant, with the call.
    pub calls: Vec<(String, &'a Expr)>,
    /// The index each quantifier the claim is inside ranges over, outermost
    /// first, with the quantifier.
    pub bound: Vec<(String, &'a Expr)>,
    pub polarity: Polarity,
    pub claim: Claim<'a>,
    pub script: String,
}
fn obligations_of<'a>(
    function: &'a Function,
    program: &'a Program,
    datatypes: &str,
) -> Vec<Obligation<'a>> {
    let ops = body_operations(function);
    let calls: Vec<Operation> = ops
        .iter()
        .filter(|op| matches!(op.expr, Expr::Call(..)))
        .cloned()
        .collect();
    let (facts, named_calls) = call_facts(program, &calls);
    let mut callees = Vec::new();
    for call in &calls {
        crate::check::callees(call.expr, &mut callees);
    }
    let prelude = format!(
        "{}{}{}{facts}",
        prelude(function, program, datatypes),
        list_facts(&[function]),
        declarations(program, &callees)
    );
    let mut params: Vec<String> = function
        .params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    params.extend(named_calls.iter().map(|(constant, _)| constant.clone()));
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
                      bound: &[&'a Expr],
                      assertion: String| {
        let (indices, assertion) = for_some(bound, assertion);
        let mut conjuncts = assumed.to_vec();
        conjuncts.push(assertion);
        let mut named = params.clone();
        named.extend(indices.iter().cloned());
        let declared: String = indices
            .iter()
            .map(|index| format!("(declare-const {index} Int)\n"))
            .collect();
        Obligation {
            function,
            functions: &program.functions,
            clause: clause.to_string(),
            line,
            params: named.clone(),
            calls: named_calls.clone(),
            bound: indices.into_iter().zip(bound.iter().copied()).collect(),
            polarity,
            claim,
            script: format!(
                "{prelude}{declare}{declared}(assert {})\n{}",
                all("and", conjuncts, "true"),
                ending(&named)
            ),
        }
    };
    let result = format!("(declare-const result {})\n", sort(&function.ret));
    let (body, covered) = value(function);
    // A claim about `result`, where `result` is the body's value.
    let of_result = |claim: String| match &covered {
        Some(covered) => format!("(and {covered} (and (= result {body}) {claim}))"),
        None => format!("(and (= result {body}) {claim})"),
    };
    let mut out = Vec::new();
    if let Some(member) = function.machine.filter(|m| m.role != Role::Observer) {
        let name = &function.name;
        let allowed = |clause: String| {
            obligation(
                &clause,
                function.line,
                Polarity::Existential,
                Claim::Allowed,
                "",
                &[],
                &[],
                "true".to_string(),
            )
        };
        let init = member.role == Role::Init;
        if init
            && function
                .clauses
                .iter()
                .any(|c| c.kind == ClauseKind::Requires)
        {
            out.push(allowed(format!("`{name}` is allowed for some input")));
        }
        // The arithmetic that makes the next state stays within its type: an
        // obligation of its own, assumed here as a call's `requires` is.
        let mut fits = every.clone();
        fits.extend(ops.iter().filter_map(range_assumption));
        let fails = format!("(not {})", meets_invariants(function));
        out.push(obligation(
            &if init {
                format!(
                    "`{name}` starts in a state that meets every invariant and helper invariant"
                )
            } else {
                format!("`{name}` preserves every invariant and helper invariant")
            },
            function.line,
            Polarity::Universal,
            Claim::Invariants,
            "",
            &fits,
            &[],
            match &covered {
                Some(covered) => format!("(and {covered} {fails})"),
                None => fails,
            },
        ));
        if !init {
            out.push(allowed(format!(
                "`{name}` is allowed in some state an init reaches"
            )));
        }
    }
    let ensures = function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures);
    for clause in ensures.clone() {
        out.push(obligation(
            &clause.text,
            clause.line,
            Polarity::Universal,
            Claim::Ensures(&clause.expr),
            &result,
            &every,
            &[],
            of_result(format!("(not {})", term(&clause.expr))),
        ));
    }
    if let (Body::Decide(rows, line), Some(covered)) = (&function.body, &covered) {
        out.push(obligation(
            "every input reaches a row",
            *line,
            Polarity::Universal,
            Claim::Covered,
            "",
            &every,
            &[],
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
                &[],
                all("and", reaches.iter().map(guard_term).collect(), "true"),
            ));
        }
    }
    for clause in ensures {
        let mut indexes = Vec::new();
        spec_indexes(&clause.expr, &[], &[], &mut indexes);
        for op in indexes {
            let bound = op.bound.clone();
            let mut negated: Vec<String> = op.guards.iter().map(guard_term).collect();
            let Some((text, line, claim, negation)) = claim_of(program, op, true) else {
                continue;
            };
            negated.push(negation);
            out.push(obligation(
                &text,
                line,
                Polarity::Universal,
                claim,
                &result,
                &every,
                &bound,
                of_result(all("and", negated, "true")),
            ));
        }
    }
    for (index, op) in ops.into_iter().enumerate() {
        let bound = op.bound.clone();
        let mut negated: Vec<String> = op.guards.iter().map(guard_term).collect();
        let Some((text, line, claim, negation)) = claim_of(program, op, false) else {
            continue;
        };
        negated.push(negation);
        out.push(obligation(
            &text,
            line,
            Polarity::Universal,
            claim,
            "",
            &before(index),
            &bound,
            all("and", negated, "true"),
        ));
    }
    // Stable, so obligations on one line keep the order above.
    out.sort_by_key(|o| o.line);
    out
}
/// The obligations of every function, in source order: one per `ensures`;
/// for a decision table, that every input reaches a row and that each row
/// can be the first to hold; that each arithmetic operation in code stays
/// in its type's range; that each call meets its callee's `requires`; and
/// that each index is in range.
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
