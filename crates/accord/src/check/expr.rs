//! The type of each expression, given the type its context expects; it
//! resolves bare variants and the width of integer literals and arithmetic.

use super::Types;
use super::calls::Signature;
use super::type_name;
use crate::Diagnostic;
use crate::syntax::{Expr, Type};

pub(super) struct Scope<'a> {
    pub types: Types<'a>,
    pub params: &'a [(String, Type)],
    pub result: Option<&'a Type>,
    /// In `requires` and `ensures`, integers are mathematical: a literal
    /// needn't fit its type.
    pub spec: bool,
    /// The functions a call may name; `None` where no call is allowed.
    pub functions: Option<&'a [Signature]>,
    /// Whether a list may be indexed here.
    pub indexing: bool,
    /// Whether this is inside a quantifier's condition.
    pub bound: bool,
}

impl Scope<'_> {
    pub(super) fn param(&self, name: &str) -> Option<&Type> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    /// Whether `name` is a function a call here may name.
    fn is_function(&self, name: &str) -> bool {
        self.functions
            .is_some_and(|functions| functions.iter().any(|f| f.name == name))
    }

    /// Whether `expr` takes its type from its context: a bare variant (a name
    /// that isn't `result` or a parameter, or a variant built like a call),
    /// an integer literal, or arithmetic made only of those.
    pub(super) fn needs_context(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name, _) => name != "result" && self.param(name).is_none(),
            Expr::Call(name, ..) => {
                !self.is_function(name)
                    && self.types.enums.iter().any(|e| e.variants.contains(name))
            }
            Expr::Int(..) => true,
            Expr::List(items, ..) => items.iter().all(|item| self.needs_context(item)),
            Expr::Neg(inner, _) => self.needs_context(inner),
            Expr::Arith(_, left, right, _) => self.needs_context(left) && self.needs_context(right),
            _ => false,
        }
    }

    /// The types of two operands that must agree, inferring the one that
    /// needs its context second.
    pub(super) fn pair(
        &self,
        left: &mut Expr,
        right: &mut Expr,
        expected: Option<&Type>,
    ) -> Result<(Type, Type), Diagnostic> {
        if self.needs_context(left) {
            let r = self.infer(right, expected)?;
            Ok((self.infer(left, Some(&r))?, r))
        } else {
            let l = self.infer(left, expected)?;
            let r = self.infer(right, Some(&l))?;
            Ok((l, r))
        }
    }

    /// The type of `expr`, given the type its context expects, if known.
    pub(super) fn infer(
        &self,
        expr: &mut Expr,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Text(..) => Ok(Type::Text),
            Expr::List(items, element, line) => {
                let known = match expected {
                    Some(Type::List(t)) => Some((**t).clone()),
                    _ => None,
                };
                let ty = match known {
                    Some(t) => t,
                    None => {
                        let Some(first) = items.iter_mut().find(|i| !self.needs_context(i)) else {
                            return Err(Diagnostic::new(
                                *line,
                                "the type of this list isn't known here",
                            ));
                        };
                        self.infer(first, None)?
                    }
                };
                for item in items.iter_mut() {
                    self.value(
                        item,
                        &ty,
                        &format!("each element must be a `{}`", type_name(&ty)),
                    )?;
                }
                *element = Some(ty.clone());
                Ok(Type::List(Box::new(ty)))
            }
            Expr::Int(value, line) => match expected {
                Some(Type::Int(width)) if self.spec || *value <= width.max() => {
                    Ok(Type::Int(*width))
                }
                Some(Type::Int(width)) => Err(Diagnostic::new(
                    *line,
                    format!("`{value}` doesn't fit `{}`", width.name()),
                )),
                Some(other) => Err(Diagnostic::new(
                    *line,
                    format!(
                        "`{value}` is an integer, but a `{}` is needed here",
                        type_name(other)
                    ),
                )),
                None => Err(Diagnostic::new(
                    *line,
                    format!("the type of `{value}` isn't known here"),
                )),
            },
            Expr::Neg(inner, site) => {
                let ty = match (inner.as_mut(), expected) {
                    // The literal after `-` may be one past the largest value.
                    (Expr::Int(value, line), Some(Type::Int(width))) => {
                        if !self.spec && -*value < width.min() {
                            return Err(Diagnostic::new(
                                *line,
                                format!("`-{value}` doesn't fit `{}`", width.name()),
                            ));
                        }
                        Type::Int(*width)
                    }
                    (inner, _) => self.infer(inner, expected)?,
                };
                match ty {
                    Type::Int(width) => {
                        site.width = Some(width);
                        Ok(ty)
                    }
                    other => Err(Diagnostic::new(
                        site.line,
                        format!("`-` needs an integer, not a `{}`", type_name(&other)),
                    )),
                }
            }
            Expr::Arith(_, left, right, site) => match self.pair(left, right, expected)? {
                (Type::Int(l), Type::Int(r)) if l == r => {
                    site.width = Some(l);
                    Ok(Type::Int(l))
                }
                _ => Err(Diagnostic::new(
                    site.line,
                    format!("both sides of `{}` need the same integer type", site.text),
                )),
            },
            Expr::Name(name, line) => {
                if name == "result" {
                    return self.result.cloned().ok_or_else(|| {
                        Diagnostic::new(*line, "`result` is only available in `ensures`")
                    });
                }
                if let Some(ty) = self.param(name) {
                    return Ok(ty.clone());
                }
                if let Some(Type::Enum(expected)) = expected
                    && self.has_variant(expected, name)
                {
                    self.fieldless(expected, name, *line)?;
                    let ty = Type::Enum(expected.clone());
                    *expr = Expr::Variant(expected.clone(), name.clone(), *line);
                    return Ok(ty);
                }
                match self.types.enums.iter().find(|e| e.variants.contains(name)) {
                    Some(owner) => Err(Diagnostic::new(
                        *line,
                        format!(
                            "the type of `{name}` isn't known here: write `{}.{name}`",
                            owner.name
                        ),
                    )),
                    None => Err(Diagnostic::new(*line, format!("unknown name `{name}`"))),
                }
            }
            // `value.field`, which reads like a qualified variant.
            Expr::Variant(owner, name, line) if self.param(owner).is_some() => {
                let value = Expr::Name(std::mem::take(owner), *line);
                *expr = Expr::Field(Box::new(value), None, std::mem::take(name), *line);
                self.infer(expr, expected)
            }
            Expr::Variant(owner, name, line) => {
                if !self.types.enums.iter().any(|e| e.name == *owner) {
                    Err(Diagnostic::new(*line, format!("unknown type `{owner}`")))
                } else if self.has_variant(owner, name) {
                    self.fieldless(owner, name, *line)?;
                    Ok(Type::Enum(owner.clone()))
                } else {
                    Err(Diagnostic::new(
                        *line,
                        format!("`{owner}` has no variant `{name}`"),
                    ))
                }
            }
            Expr::Not(inner) => {
                self.condition(inner)?;
                Ok(Type::Bool)
            }
            Expr::Binary(op, left, right) => self.binary(*op, left, right),
            Expr::Call(name, args, site) => {
                let line = site.line;
                if let Some(Type::Enum(owner)) = expected
                    && self.has_variant(owner, name)
                {
                    *expr = Expr::Construct(
                        owner.clone(),
                        std::mem::take(name),
                        std::mem::take(args),
                        line,
                    );
                    return self.infer(expr, expected);
                }
                if !self.is_function(name)
                    && let Some(owner) = self.types.enums.iter().find(|e| e.variants.contains(name))
                {
                    return Err(Diagnostic::new(
                        line,
                        format!(
                            "the type of `{name}` isn't known here: write `{}.{name}(…)`",
                            owner.name
                        ),
                    ));
                }
                self.call(name, args, line)
            }
            Expr::Construct(owner, variant, args, line) => {
                self.construct(owner, variant, args, *line)
            }
            Expr::Record(name, fields, line) => self.record_value(name, fields, *line),
            Expr::Field(value, owner, field, line) => self.field(value, owner, field, *line),
            Expr::Match(value, arms, line) => self.matched(value, arms, expected, *line),
            Expr::Is(..) => Ok(Type::Bool),
            Expr::Quantified(quantifier, name, range, condition, line) => {
                self.quantified(*quantifier, name, range, condition, *line)
            }
            Expr::Length(list, line) => self.length(list, *line),
            Expr::Next(_, line) => Err(Diagnostic::new(
                *line,
                "`next` can only end an init or an event",
            )),
            Expr::Index(list, index, site) => self.index(list, index, site.line),
            Expr::If(condition, then, otherwise) => {
                self.condition(condition)?;
                let a = self.infer(then, expected)?;
                let b = self.infer(otherwise, expected.or(Some(&a)))?;
                if a == b {
                    Ok(a)
                } else {
                    Err(Diagnostic::new(
                        condition.line(),
                        "both branches of `if` need the same type",
                    ))
                }
            }
        }
    }

    pub(super) fn has_variant(&self, owner: &str, name: &str) -> bool {
        self.types
            .enums
            .iter()
            .any(|e| e.name == owner && e.variants.iter().any(|v| v == name))
    }

    pub(super) fn condition(&self, expr: &mut Expr) -> Result<(), Diagnostic> {
        match self.infer(expr, Some(&Type::Bool))? {
            Type::Bool => Ok(()),
            _ => Err(Diagnostic::new(expr.line(), "expected a `Bool` condition")),
        }
    }

    pub(super) fn value(
        &self,
        expr: &mut Expr,
        ty: &Type,
        message: &str,
    ) -> Result<(), Diagnostic> {
        if self.infer(expr, Some(ty))? == *ty {
            Ok(())
        } else {
            Err(Diagnostic::new(expr.line(), message))
        }
    }
}
