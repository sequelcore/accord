//! What each obligation claims, and the claim an operation carries.

use super::calls::{has_requires, requires_of};
use super::lists::in_bounds;
use super::operations::{Guard, Operation};
use super::smt::{in_range, term};
use crate::syntax::{Expr, Program, Width};

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
    /// Where the guards hold, `index`'s index is in range; in an `ensures`,
    /// with `result` the body's value.
    InBounds {
        index: &'a Expr,
        guards: Vec<Guard<'a>>,
        ensures: bool,
    },
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
