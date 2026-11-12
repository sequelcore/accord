//! Name and type checks that run before any obligation is generated. They
//! also resolve what needs the expected type: each bare variant's enum, and
//! the width of each integer literal and arithmetic operation.

mod calls;
mod example;
mod expr;
mod lists;
mod operators;

use calls::{Signature, check_recursion, ordered};
pub(crate) use calls::{body_callees, callees};
use expr::Scope;

use crate::Diagnostic;
use crate::syntax::{Body, ClauseKind, Enum, Function, Program, Type};

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int(width) => width.name().to_string(),
        Type::Text => "Text".to_string(),
        Type::List(element) => format!("List<{}>", type_name(element)),
        Type::Enum(name) => name.clone(),
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
            let text = e.text(variant);
            if let Some(other) = e.variants[..i].iter().find(|o| e.text(o) == text) {
                return Err(Diagnostic::new(
                    e.line,
                    format!(
                        "`{other}` and `{variant}` of `{}` are both \"{text}\"",
                        e.name
                    ),
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
            indexing: clause.kind == ClauseKind::Ensures,
            bound: false,
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
        indexing: true,
        bound: false,
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
