//! Accord expressions as TypeScript expressions.
//!
//! In code that runs, `i32` is a `number` and `i64` a `bigint`; the proofs
//! that no operation overflows make `number` arithmetic exact for `i32`. In a
//! specification every integer is a `bigint`, since specification arithmetic
//! is mathematical.

use crate::syntax::{Arith, Expr, Op, Type, Width};

/// Where an expression is evaluated.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Context {
    /// The body: fixed-width integers.
    Code,
    /// A `requires`, checked at the boundary: mathematical integers.
    Spec,
}

/// What an expression's names mean.
pub(super) struct Scope<'a> {
    pub params: &'a [(String, Type)],
    pub context: Context,
}

/// `snake_case` as `camelCase`.
pub(super) fn camel(name: &str) -> String {
    let mut out = String::new();
    for (index, part) in name.split('_').enumerate() {
        let mut chars = part.chars();
        match chars.next() {
            Some(first) if index > 0 => {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
            _ => out.push_str(part),
        }
    }
    out
}

/// A variant's `PascalCase` name as its `snake_case` string value.
pub(super) fn value(variant: &str) -> String {
    let mut out = String::new();
    for (index, c) in variant.chars().enumerate() {
        if c.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    format!("\"{out}\"")
}

/// A text as a TypeScript string literal.
pub(super) fn string(text: &str) -> String {
    serde_json::to_string(text).expect("a string serializes")
}

impl Scope<'_> {
    fn param(&self, name: &str) -> Option<&Type> {
        self.params
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, ty)| ty)
    }

    /// The integer width of `expr`, where it has one of its own.
    fn width(&self, expr: &Expr) -> Option<Width> {
        match expr {
            Expr::Name(name, _) => match self.param(name) {
                Some(Type::Int(width)) => Some(*width),
                _ => None,
            },
            Expr::Arith(.., site) | Expr::Neg(_, site) => site.width,
            Expr::If(_, then, otherwise) => self.width(then).or_else(|| self.width(otherwise)),
            _ => None,
        }
    }

    fn is_list(&self, expr: &Expr) -> bool {
        match expr {
            Expr::List(..) => true,
            Expr::Name(name, _) => matches!(self.param(name), Some(Type::List(_))),
            _ => false,
        }
    }

    /// `expr`, parenthesized when it is an operator applied to operands.
    fn wrapped(&self, expr: &Expr, want: Option<Width>) -> String {
        let operator = match expr {
            Expr::Binary(op, ..) => *op != Op::In,
            Expr::Arith(..) | Expr::If(..) => true,
            _ => false,
        };
        let bare = self.bare(expr, want);
        if operator { format!("({bare})") } else { bare }
    }

    /// `expr` with no parentheses around it; integer literals take the
    /// representation of `want`.
    pub(super) fn bare(&self, expr: &Expr, want: Option<Width>) -> String {
        match expr {
            Expr::Bool(b) => b.to_string(),
            Expr::Int(n, _) => match (self.context, want) {
                (Context::Spec, _) | (Context::Code, Some(Width::I64)) => format!("{n}n"),
                (Context::Code, _) => n.to_string(),
            },
            Expr::Text(text, _) => string(text),
            Expr::List(items, ..) => {
                let items: Vec<String> = items.iter().map(|i| self.bare(i, None)).collect();
                format!("[{}]", items.join(", "))
            }
            Expr::Name(name, _) => match (self.context, self.param(name)) {
                (Context::Spec, Some(Type::Int(_))) => format!("BigInt({})", camel(name)),
                _ => camel(name),
            },
            Expr::Variant(_, variant, _) => value(variant),
            Expr::Not(inner) => format!("!{}", self.wrapped(inner, None)),
            Expr::Neg(inner, site) => format!("-{}", self.wrapped(inner, site.width)),
            Expr::Arith(arith, left, right, site) => {
                let symbol = match arith {
                    Arith::Add => "+",
                    Arith::Sub => "-",
                    Arith::Mul => "*",
                };
                let (l, r) = (
                    self.wrapped(left, site.width),
                    self.wrapped(right, site.width),
                );
                format!("{l} {symbol} {r}")
            }
            Expr::If(condition, then, otherwise) => format!(
                "{} ? {} : {}",
                self.wrapped(condition, None),
                self.wrapped(then, want),
                self.wrapped(otherwise, want)
            ),
            Expr::Binary(op, left, right) => self.binary(*op, left, right),
        }
    }

    fn binary(&self, op: Op, left: &Expr, right: &Expr) -> String {
        if op == Op::In {
            return format!(
                "{}.includes({})",
                self.wrapped(right, None),
                self.bare(left, None)
            );
        }
        if matches!(op, Op::Eq | Op::Ne) && (self.is_list(left) || self.is_list(right)) {
            let (a, b) = (self.wrapped(left, None), self.wrapped(right, None));
            let equal = format!(
                "{a}.length === {b}.length && {a}.every((item, index) => item === {b}[index])"
            );
            return if op == Op::Eq {
                equal
            } else {
                format!("!({equal})")
            };
        }
        let width = self.width(left).or_else(|| self.width(right));
        let (l, r) = (self.wrapped(left, width), self.wrapped(right, width));
        let symbol = match op {
            Op::And => "&&",
            Op::Or => "||",
            Op::Implies => return format!("!{l} || {r}"),
            Op::Eq => "===",
            Op::Ne => "!==",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::In => unreachable!("handled above"),
        };
        format!("{l} {symbol} {r}")
    }
}

#[cfg(test)]
mod tests {
    use super::{camel, value};

    #[test]
    fn names_follow_typescript_conventions() {
        assert_eq!(camel("timed_out"), "timedOut");
        assert_eq!(camel("proof_outcome"), "proofOutcome");
        assert_eq!(camel("n"), "n");
        assert_eq!(value("TimedOut"), "\"timed_out\"");
        assert_eq!(value("Red"), "\"red\"");
    }
}
