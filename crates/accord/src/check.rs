//! Name and type checks that run before any obligation is generated. They
//! also resolve what needs the expected type: each bare variant's enum, and
//! the width of each integer literal and arithmetic operation.

mod example;

use crate::Diagnostic;
use crate::syntax::{Body, ClauseKind, Enum, Expr, Op, Program, Type};

struct Scope<'a> {
    pub enums: &'a [Enum],
    pub params: &'a [(String, Type)],
    pub result: Option<&'a Type>,
    /// In `requires` and `ensures`, integers are mathematical: a literal
    /// needn't fit its type.
    pub spec: bool,
}

fn type_name(ty: &Type) -> &str {
    match ty {
        Type::Bool => "Bool",
        Type::Int(width) => width.name(),
        Type::Text => "Text",
        Type::Enum(name) => name,
    }
}

impl Scope<'_> {
    fn param(&self, name: &str) -> Option<&Type> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    /// Whether `expr` takes its type from its context: a bare variant (a name
    /// that isn't `result` or a parameter), an integer literal, or arithmetic
    /// made only of those.
    fn needs_context(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name, _) => name != "result" && self.param(name).is_none(),
            Expr::Int(..) => true,
            Expr::Neg(inner, _) => self.needs_context(inner),
            Expr::Arith(_, left, right, _) => self.needs_context(left) && self.needs_context(right),
            _ => false,
        }
    }

    /// The types of two operands that must agree, inferring the one that
    /// needs its context second.
    fn pair(
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
    fn infer(&self, expr: &mut Expr, expected: Option<&Type>) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Text(..) => Ok(Type::Text),
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
                    let ty = Type::Enum(expected.clone());
                    *expr = Expr::Variant(expected.clone(), name.clone(), *line);
                    return Ok(ty);
                }
                match self.enums.iter().find(|e| e.variants.contains(name)) {
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
            Expr::Variant(owner, name, line) => {
                if !self.enums.iter().any(|e| e.name == *owner) {
                    Err(Diagnostic::new(*line, format!("unknown type `{owner}`")))
                } else if self.has_variant(owner, name) {
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
            Expr::Binary(op, left, right) => {
                let line = line_of(left);
                match op {
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
            Expr::If(condition, then, otherwise) => {
                self.condition(condition)?;
                let a = self.infer(then, expected)?;
                let b = self.infer(otherwise, expected.or(Some(&a)))?;
                if a == b {
                    Ok(a)
                } else {
                    Err(Diagnostic::new(
                        line_of(condition),
                        "both branches of `if` need the same type",
                    ))
                }
            }
        }
    }

    fn has_variant(&self, owner: &str, name: &str) -> bool {
        self.enums
            .iter()
            .any(|e| e.name == owner && e.variants.iter().any(|v| v == name))
    }

    fn condition(&self, expr: &mut Expr) -> Result<(), Diagnostic> {
        match self.infer(expr, Some(&Type::Bool))? {
            Type::Bool => Ok(()),
            _ => Err(Diagnostic::new(
                line_of(expr),
                "expected a `Bool` condition",
            )),
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
            Err(Diagnostic::new(line_of(expr), message))
        }
    }
}

fn line_of(expr: &Expr) -> usize {
    match expr {
        Expr::Name(_, line)
        | Expr::Variant(_, _, line)
        | Expr::Int(_, line)
        | Expr::Text(_, line) => *line,
        Expr::Arith(.., site) | Expr::Neg(_, site) => site.line,
        Expr::Not(inner) => line_of(inner),
        Expr::Binary(_, left, _) | Expr::If(left, _, _) => line_of(left),
        Expr::Bool(_) => 0,
    }
}

fn check_enums(enums: &[Enum]) -> Result<(), Diagnostic> {
    for (index, e) in enums.iter().enumerate() {
        if e.name == "Bool" || enums[..index].iter().any(|o| o.name == e.name) {
            return Err(Diagnostic::new(
                e.line,
                format!("the type `{}` is already defined", e.name),
            ));
        }
        for (i, variant) in e.variants.iter().enumerate() {
            if e.variants[..i].contains(variant) {
                return Err(Diagnostic::new(
                    e.line,
                    format!("`{}` lists `{variant}` twice", e.name),
                ));
            }
        }
    }
    Ok(())
}

fn check_type(enums: &[Enum], ty: &Type, line: usize) -> Result<(), Diagnostic> {
    match ty {
        Type::Enum(name) if !enums.iter().any(|e| e.name == *name) => {
            Err(Diagnostic::new(line, format!("unknown type `{name}`")))
        }
        _ => Ok(()),
    }
}

/// Checks names and types in every function and example, and resolves bare
/// variants.
pub(crate) fn check(program: &mut Program) -> Result<(), Diagnostic> {
    check_enums(&program.enums)?;
    let enums = &program.enums;
    for function in &mut program.functions {
        for (_, ty) in &function.params {
            check_type(enums, ty, function.line)?;
        }
        check_type(enums, &function.ret, function.line)?;
        for clause in &mut function.clauses {
            let scope = Scope {
                enums,
                params: &function.params,
                result: (clause.kind == ClauseKind::Ensures).then_some(&function.ret),
                spec: true,
            };
            if scope.infer(&mut clause.expr, Some(&Type::Bool))? != Type::Bool {
                return Err(Diagnostic::new(
                    clause.line,
                    "a clause must be a `Bool` condition",
                ));
            }
        }
        let scope = Scope {
            enums,
            params: &function.params,
            result: None,
            spec: false,
        };
        let message = "the body's type isn't the function's return type";
        match &mut function.body {
            Body::Expr(body) => scope.value(body, &function.ret, message)?,
            Body::Decide(rows, _) => {
                for row in rows {
                    if let Some(condition) = &mut row.condition {
                        scope.condition(condition)?;
                    }
                    scope.value(&mut row.result, &function.ret, message)?;
                }
            }
        }
    }
    let functions = &program.functions;
    for example in &mut program.examples {
        example::check(enums, functions, example)?;
    }
    Ok(())
}
