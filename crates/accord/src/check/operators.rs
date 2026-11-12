//! Binary operators: membership, comparisons and connectives.

use super::expr::{Scope, line_of};
use super::type_name;
use crate::Diagnostic;
use crate::syntax::{Expr, Op, Type};

impl Scope<'_> {
    /// The type of `left op right`.
    pub(super) fn binary(
        &self,
        op: Op,
        left: &mut Expr,
        right: &mut Expr,
    ) -> Result<Type, Diagnostic> {
        // A `true` or `false` carries no line; the other side does.
        let line = match line_of(left) {
            0 => line_of(right),
            line => line,
        };
        match op {
            Op::In => {
                let (element, list) = if self.needs_context(right) {
                    let element = self.infer(left, None)?;
                    let list = Type::List(Box::new(element.clone()));
                    (element, self.infer(right, Some(&list))?)
                } else {
                    let list = self.infer(right, None)?;
                    let element = match &list {
                        Type::List(t) => Some((**t).clone()),
                        _ => None,
                    };
                    (self.infer(left, element.as_ref())?, list)
                };
                if list == Type::List(Box::new(element.clone())) {
                    Ok(Type::Bool)
                } else {
                    Err(Diagnostic::new(
                        line,
                        format!(
                            "`in` needs a list of `{}` on its right",
                            type_name(&element)
                        ),
                    ))
                }
            }
            Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let (l, r) = self.pair(left, right, None)?;
                let ordered = !matches!(op, Op::Eq | Op::Ne);
                if l != r {
                    Err(Diagnostic::new(
                        line,
                        "both sides of a comparison need the same type",
                    ))
                } else if ordered && !matches!(l, Type::Int(_)) {
                    Err(Diagnostic::new(line, "only integers can be ordered"))
                } else {
                    Ok(Type::Bool)
                }
            }
            Op::And | Op::Or | Op::Implies => {
                self.condition(left)?;
                self.condition(right)?;
                Ok(Type::Bool)
            }
        }
    }
}
