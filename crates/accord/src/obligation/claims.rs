//! What each obligation claims, and the claim an operation carries.

use super::calls::{has_requires, requires_of};
use super::lists::in_bounds;
use super::operations::{Guard, Operation};
use super::smt::{in_range, term};
use crate::syntax::{ClauseKind, Expr, Function, Program, Width};

/// What an obligation states, for admitted inputs (every `requires` holds).
pub(crate) enum Claim<'a> {
    /// The `ensures` holds of the body's value.
    Ensures(&'a Expr),
    /// The state a machine's init or event ends in meets every invariant and
    /// helper invariant.
    Invariants,
    /// A machine's init has some input its `requires` admits, or its event
    /// is allowed in some state that meets the invariants.
    Allowed,
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
    /// Where the guards hold, the function's `measure` at `call`'s arguments
    /// is non-negative and smaller than at its own parameters.
    Decreases {
        call: &'a Expr,
        measure: &'a Expr,
        guards: Vec<Guard<'a>>,
    },
    /// Where the guards hold, `index`'s index is in range; in an `ensures`,
    /// with `result` the body's value.
    InBounds {
        index: &'a Expr,
        guards: Vec<Guard<'a>>,
        ensures: bool,
    },
}
/// What a call `function` makes to itself claims of its `decreases`
/// measure, with its clause and line, and the claim's negation; `None` for
/// any other operation.
pub(super) fn termination_of<'a>(
    function: &'a Function,
    op: &Operation<'a>,
) -> Option<(String, usize, Claim<'a>, String)> {
    let Expr::Call(name, args, site) = op.expr else {
        return None;
    };
    let measure = function
        .clauses
        .iter()
        .find(|c| c.kind == ClauseKind::Decreases)?;
    if *name != function.name {
        return None;
    }
    let old = term(&measure.expr);
    let bindings: Vec<String> = function
        .params
        .iter()
        .zip(args)
        .map(|((param, _), (_, arg))| format!("({param} {})", term(arg)))
        .collect();
    let new = if bindings.is_empty() {
        old.clone()
    } else {
        format!("(let ({}) {old})", bindings.join(" "))
    };
    Some((
        format!("`{}` meets `{}`", site.text, measure.text),
        site.line,
        Claim::Decreases {
            call: op.expr,
            measure: &measure.expr,
            guards: op.guards.clone(),
        },
        format!("(not (and (<= 0 {new}) (< {new} {old})))"),
    ))
}

/// What an operation claims, with its clause and line, and the claim's
/// negation; `None` for one that claims nothing.
pub(super) fn claim_of<'a>(
    program: &Program,
    op: Operation<'a>,
    ensures: bool,
) -> Option<(String, usize, Claim<'a>, String)> {
    let Operation { expr, guards, .. } = op;
    Some(match expr {
        Expr::Arith(.., site) | Expr::Neg(_, site) => {
            let Some(width) = site.width else {
                // `check` sets every width before obligations are generated.
                unreachable!("arithmetic without a width: `{}`", site.text)
            };
            (
                format!("`{}` stays within `{}`", site.text, width.name()),
                site.line,
                Claim::InRange {
                    expr,
                    width,
                    guards,
                },
                format!("(not {})", in_range(&term(expr), width)),
            )
        }
        Expr::Call(name, _, site) if has_requires(program, expr) => {
            // The call as written starts with its callee's source name;
            // `name` may be normalized.
            let callee = site.text.split('(').next().unwrap_or(name).trim();
            (
                format!("`{}` meets `{callee}`'s requires", site.text),
                site.line,
                Claim::Requires { call: expr, guards },
                format!("(not {})", requires_of(program, expr)),
            )
        }
        Expr::Index(list, index, site) => (
            format!("`{}` is within its list", site.text),
            site.line,
            Claim::InBounds {
                index: expr,
                guards,
                ensures,
            },
            format!("(not {})", in_bounds(&term(list), &term(index))),
        ),
        _ => return None,
    })
}
