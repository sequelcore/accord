//! Quantifiers over lists, `length` and indexing.

use super::expr::Scope;
use super::type_name;
use crate::Diagnostic;
use crate::syntax::{Expr, Quantifier, Range, Type, Width};

impl Scope<'_> {
    /// The type of `quantifier name in range: condition`: `Bool` for `all`
    /// and `any`, `i64` for `count`.
    pub(super) fn quantified(
        &self,
        quantifier: Quantifier,
        name: &str,
        range: &mut Range,
        condition: &mut Expr,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        if quantifier == Quantifier::Count && self.bound {
            return Err(Diagnostic::new(
                line,
                "`count` can't be inside another quantifier yet",
            ));
        }
        if name == "result" || self.params.iter().any(|(n, _)| n == name) {
            return Err(Diagnostic::new(
                line,
                format!("`{name}` is already a name here"),
            ));
        }
        let element = match self.infer(range.list_mut(), None)? {
            Type::List(element) => *element,
            other => {
                return Err(Diagnostic::new(
                    line,
                    format!(
                        "`{}` ranges over a list, not a `{}`",
                        quantifier.name(),
                        type_name(&other)
                    ),
                ));
            }
        };
        let bound = match range {
            Range::Elements(_) => element,
            Range::Indices(_) => Type::Int(Width::I64),
        };
        let mut params = self.params.to_vec();
        params.push((name.to_string(), bound));
        let inner = Scope {
            params: &params,
            bound: true,
            ..*self
        };
        inner.condition(condition)?;
        Ok(match quantifier {
            Quantifier::Count => Type::Int(Width::I64),
            Quantifier::All | Quantifier::Any => Type::Bool,
        })
    }

    /// The type of `length(list)`: `i64`.
    pub(super) fn length(&self, list: &mut Expr, line: usize) -> Result<Type, Diagnostic> {
        match self.infer(list, None)? {
            Type::List(_) => Ok(Type::Int(Width::I64)),
            other => Err(Diagnostic::new(
                line,
                format!("`length` needs a list, not a `{}`", type_name(&other)),
            )),
        }
    }

    /// The type of `list[index]`: the list's element type.
    pub(super) fn index(
        &self,
        list: &mut Expr,
        index: &mut Expr,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        if !self.indexing {
            return Err(Diagnostic::new(
                line,
                if self.spec {
                    "a `requires` or `open when` can't index a list yet"
                } else {
                    "an example's arguments can't index a list"
                },
            ));
        }
        let element = match self.infer(list, None)? {
            Type::List(element) => *element,
            other => {
                return Err(Diagnostic::new(
                    list.line(),
                    format!("only a list can be indexed, not a `{}`", type_name(&other)),
                ));
            }
        };
        self.value(index, &Type::Int(Width::I64), "an index must be an `i64`")?;
        Ok(element)
    }
}
