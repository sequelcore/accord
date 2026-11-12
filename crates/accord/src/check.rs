//! Name and type checks that run before any obligation is generated. They
//! also resolve what needs the expected type: each bare variant's enum, and
//! the width of each integer literal and arithmetic operation.

mod example;

use crate::Diagnostic;
use crate::syntax::{Body, ClauseKind, Enum, Expr, Function, Op, Program, Type};

/// What a call needs to know about the function it calls.
pub(super) struct Signature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
}

struct Scope<'a> {
    pub enums: &'a [Enum],
    pub params: &'a [(String, Type)],
    pub result: Option<&'a Type>,
    /// In `requires` and `ensures`, integers are mathematical: a literal
    /// needn't fit its type.
    pub spec: bool,
    /// The functions a call may name; `None` where no call is allowed.
    pub functions: Option<&'a [Signature]>,
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int(width) => width.name().to_string(),
        Type::Text => "Text".to_string(),
        Type::List(element) => format!("List<{}>", type_name(element)),
        Type::Enum(name) => name.clone(),
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
            Expr::List(items, ..) => items.iter().all(|item| self.needs_context(item)),
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
            Expr::Call(name, args, site) => self.call(name, args, site.line),
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

    /// Checks a call's arguments against the function it names, puts them in
    /// the order of its parameters, each labelled, and gives its result type.
    fn call(
        &self,
        name: &str,
        args: &mut Vec<(Option<String>, Expr)>,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let Some(functions) = self.functions else {
            return Err(Diagnostic::new(
                line,
                if self.spec {
                    "a specification can't call a function yet"
                } else {
                    "an example's arguments can't call a function"
                },
            ));
        };
        let Some(callee) = functions.iter().find(|f| f.name == name) else {
            return Err(Diagnostic::new(line, format!("unknown function `{name}`")));
        };
        let ordered = ordered(name, &callee.params, std::mem::take(args), line)?;
        for ((param, ty), mut expr) in callee.params.iter().zip(ordered) {
            self.value(
                &mut expr,
                ty,
                &format!("`{param}` needs a `{}`", type_name(ty)),
            )?;
            args.push((Some(param.clone()), expr));
        }
        Ok(callee.ret.clone())
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

/// A call's arguments in the order of `params`, from arguments that are
/// labelled, or a single one that may not be.
pub(super) fn ordered(
    function: &str,
    params: &[(String, Type)],
    args: Vec<(Option<String>, Expr)>,
    line: usize,
) -> Result<Vec<Expr>, Diagnostic> {
    let count = params.len();
    if args.len() != count {
        return Err(Diagnostic::new(
            line,
            format!("`{function}` takes {count} arguments"),
        ));
    }
    if count > 1 && args.iter().any(|(label, _)| label.is_none()) {
        return Err(Diagnostic::new(
            line,
            format!("label every argument: `{function}` has more than one parameter"),
        ));
    }
    let mut ordered: Vec<Option<Expr>> = vec![None; count];
    for (label, expr) in args {
        let index = match &label {
            None => 0,
            Some(label) => params
                .iter()
                .position(|(name, _)| name == label)
                .ok_or_else(|| {
                    Diagnostic::new(line, format!("`{function}` has no parameter `{label}`"))
                })?,
        };
        if ordered[index].replace(expr).is_some() {
            return Err(Diagnostic::new(
                line,
                format!("`{}` is given twice", params[index].0),
            ));
        }
    }
    params
        .iter()
        .zip(ordered)
        .map(|((name, _), expr)| {
            expr.ok_or_else(|| Diagnostic::new(line, format!("`{name}` is missing")))
        })
        .collect()
}

/// The names of the functions `expr` calls, inner calls first, each once.
pub(crate) fn callees(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Call(name, args, _) => {
            for (_, arg) in args {
                callees(arg, out);
            }
            if !out.contains(name) {
                out.push(name.clone());
            }
        }
        Expr::Not(inner) | Expr::Neg(inner, _) => callees(inner, out),
        Expr::Binary(_, left, right) | Expr::Arith(_, left, right, _) => {
            callees(left, out);
            callees(right, out);
        }
        Expr::If(condition, then, otherwise) => {
            callees(condition, out);
            callees(then, out);
            callees(otherwise, out);
        }
        Expr::List(items, ..) => items.iter().for_each(|item| callees(item, out)),
        Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
}

/// The names of the functions `function`'s body calls, each once.
pub(crate) fn body_callees(function: &Function) -> Vec<String> {
    let mut out = Vec::new();
    match &function.body {
        Body::Expr(body) => callees(body, &mut out),
        Body::Decide(rows, _) => {
            for row in rows {
                if let Some(condition) = &row.condition {
                    callees(condition, &mut out);
                }
                callees(&row.result, &mut out);
            }
        }
    }
    out
}

/// Rejects a function that calls itself, directly or through others, since
/// termination isn't proved yet.
fn check_recursion(program: &Program) -> Result<(), Diagnostic> {
    for function in &program.functions {
        let mut seen: Vec<String> = Vec::new();
        let mut pending = body_callees(function);
        while let Some(name) = pending.pop() {
            if name == function.name {
                return Err(Diagnostic::new(
                    function.line,
                    format!(
                        "`{}` calls itself, directly or through another function: recursion isn't supported yet",
                        function.name
                    ),
                ));
            }
            if seen.contains(&name) {
                continue;
            }
            if let Some(callee) = program.functions.iter().find(|f| f.name == name) {
                pending.extend(body_callees(callee));
            }
            seen.push(name);
        }
    }
    Ok(())
}

fn line_of(expr: &Expr) -> usize {
    match expr {
        Expr::Name(_, line)
        | Expr::Variant(_, _, line)
        | Expr::Int(_, line)
        | Expr::Text(_, line)
        | Expr::List(_, _, line) => *line,
        Expr::Arith(.., site) | Expr::Neg(_, site) | Expr::Call(.., site) => site.line,
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
        Type::List(element) => match **element {
            Type::Int(_) | Type::List(_) => Err(Diagnostic::new(
                line,
                format!("lists of `{}` aren't supported yet", type_name(element)),
            )),
            _ => check_type(enums, element, line),
        },
        Type::Enum(name) if !enums.iter().any(|e| e.name == *name) => {
            Err(Diagnostic::new(line, format!("unknown type `{name}`")))
        }
        _ => Ok(()),
    }
}

/// Every function's name once across the folder.
fn check_names(program: &Program) -> Result<(), Diagnostic> {
    for (index, f) in program.functions.iter().enumerate() {
        if program.functions[..index].iter().any(|o| o.name == f.name) {
            return Err(Diagnostic::new(
                f.line,
                format!("`{}` is declared more than once", f.name),
            ));
        }
    }
    Ok(())
}

/// Checks names and types in every function and example, and resolves bare
/// variants. A diagnostic's line is in the file of the declaration it names,
/// which `files` gives by index.
pub(crate) fn check(program: &mut Program, files: &[String]) -> Result<(), Diagnostic> {
    let name = |file: usize| files.get(file).cloned().unwrap_or_default();
    let enum_file = |line: usize, p: &Program| {
        p.enums
            .iter()
            .find(|e| e.line == line)
            .map_or(0, |e| e.file)
    };
    check_enums(&program.enums).map_err(|d| {
        let file = enum_file(d.line, program);
        d.in_file(&name(file))
    })?;
    check_names(program).map_err(|d| {
        let file = program
            .functions
            .iter()
            .rev()
            .find(|f| f.line == d.line)
            .map_or(0, |f| f.file);
        d.in_file(&name(file))
    })?;
    let enums = &program.enums;
    let signatures: Vec<Signature> = program
        .functions
        .iter()
        .map(|f| Signature {
            name: f.name.clone(),
            params: f.params.clone(),
            ret: f.ret.clone(),
        })
        .collect();
    for function in &mut program.functions {
        let file = name(function.file);
        check_function(enums, &signatures, function).map_err(|d| d.in_file(&file))?;
    }
    check_recursion(program).map_err(|d| {
        let file = program
            .functions
            .iter()
            .find(|f| f.line == d.line)
            .map_or(0, |f| f.file);
        d.in_file(&name(file))
    })?;
    let functions = &program.functions;
    for example in &mut program.examples {
        let file = name(example.file);
        example::check(enums, functions, example).map_err(|d| d.in_file(&file))?;
    }
    Ok(())
}

fn check_function(
    enums: &[Enum],
    signatures: &[Signature],
    function: &mut Function,
) -> Result<(), Diagnostic> {
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
            functions: None,
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
        functions: Some(signatures),
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
    Ok(())
}
