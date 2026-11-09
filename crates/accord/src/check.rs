//! Name and type checks that run before any obligation is generated. They
//! also resolve each bare variant to its enum, which needs the expected type.

use crate::Diagnostic;
use crate::syntax::{Body, ClauseKind, Enum, Expr, Op, Program, Type};

struct Scope<'a> {
    enums: &'a [Enum],
    params: &'a [(String, Type)],
    result: Option<&'a Type>,
}

impl Scope<'_> {
    fn param(&self, name: &str) -> Option<&Type> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    /// A name that can only be a variant: not `result` and not a parameter.
    fn is_bare_variant(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Name(name, _) if name != "result" && self.param(name).is_none())
    }

    /// The type of `expr`, given the type its context expects, if known.
    fn infer(&self, expr: &mut Expr, expected: Option<&Type>) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Bool(_) => Ok(Type::Bool),
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
                    Op::Eq | Op::Ne => {
                        // A bare variant takes its type from the other side.
                        let (l, r) = if self.is_bare_variant(left) {
                            let r = self.infer(right, None)?;
                            (self.infer(left, Some(&r))?, r)
                        } else {
                            let l = self.infer(left, None)?;
                            let r = self.infer(right, Some(&l))?;
                            (l, r)
                        };
                        if l == r {
                            Ok(Type::Bool)
                        } else {
                            Err(Diagnostic::new(
                                line,
                                "both sides of a comparison need the same type",
                            ))
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
            Type::Enum(_) => Err(Diagnostic::new(
                line_of(expr),
                "expected a `Bool` condition",
            )),
        }
    }

    fn value(&self, expr: &mut Expr, ty: &Type, message: &str) -> Result<(), Diagnostic> {
        if self.infer(expr, Some(ty))? == *ty {
            Ok(())
        } else {
            Err(Diagnostic::new(line_of(expr), message))
        }
    }
}

fn line_of(expr: &Expr) -> usize {
    match expr {
        Expr::Name(_, line) | Expr::Variant(_, _, line) => *line,
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

/// Checks names and types in every function, and resolves bare variants.
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
    Ok(())
}
