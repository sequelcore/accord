//! Quantifiers, `length` and indexing as TypeScript: `every`, `some` and
//! `filter` over an array. An index a quantifier ranges over is a `number`
//! there, though an `i64` in Accord, so it is read as a `bigint` except to
//! index an array.

use super::expr::Scope;
use crate::syntax::{Expr, Quantifier, Range, Type, Width};

impl Scope<'_> {
    /// The type of a list's elements, where it is known.
    fn element(&self, list: &Expr) -> Type {
        let known = match list {
            Expr::List(_, element, _) => element.clone(),
            Expr::Name(name, _) => match self.param(name) {
                Some(Type::List(element)) => Some((**element).clone()),
                _ => None,
            },
            _ => None,
        };
        // Only a width matters here, and no list holds integers.
        known.unwrap_or(Type::Bool)
    }

    pub(super) fn quantified(
        &self,
        quantifier: Quantifier,
        name: &str,
        range: &Range,
        condition: &Expr,
    ) -> String {
        let list = self.wrapped(range.list(), None);
        let mut params = self.params.to_vec();
        let mut indices = self.indices.to_vec();
        let binding = match range {
            Range::Elements(items) => {
                params.push((name.to_string(), self.element(items)));
                format!("({})", super::camel(name))
            }
            Range::Indices(_) => {
                params.push((name.to_string(), Type::Int(Width::I64)));
                indices.push(name.to_string());
                format!("(_, {})", super::camel(name))
            }
        };
        let inner = Scope {
            params: &params,
            indices: &indices,
            ..*self
        };
        let condition = inner.bare(condition, None);
        match quantifier {
            Quantifier::All => format!("{list}.every({binding} => {condition})"),
            Quantifier::Any => format!("{list}.some({binding} => {condition})"),
            Quantifier::Count => format!("BigInt({list}.filter({binding} => {condition}).length)"),
        }
    }

    pub(super) fn length(&self, list: &Expr) -> String {
        format!("BigInt({}.length)", self.wrapped(list, None))
    }

    pub(super) fn index(&self, list: &Expr, index: &Expr) -> String {
        let at = match index {
            Expr::Name(name, _) if self.indices.contains(name) => super::camel(name),
            _ => format!("Number({})", self.bare(index, Some(Width::I64))),
        };
        format!("{}[{at}]", self.wrapped(list, None))
    }

    /// Whether `name` is an index a quantifier ranges over.
    pub(super) fn is_index(&self, name: &str) -> bool {
        self.indices.iter().any(|n| n == name)
    }
}
