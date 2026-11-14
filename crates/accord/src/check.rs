//! Name and type checks that run before any obligation is generated. They
//! also resolve what needs the expected type: each bare variant's enum, and
//! the width of each integer literal and arithmetic operation.

mod calls;
mod example;
mod expr;
mod lists;
mod machine;
mod operators;
mod structured;

use calls::{Signature, check_recursion, ordered};
pub(crate) use calls::{body_callees, callees};
use expr::Scope;

use crate::Diagnostic;
use crate::syntax::{Body, ClauseKind, Enum, Function, Program, Record, Type};

fn type_name(ty: &Type) -> String {
    ty.name()
}

/// The types a program declares.
#[derive(Clone, Copy)]
pub(super) struct Types<'a> {
    pub enums: &'a [Enum],
    pub records: &'a [Record],
}

impl Types<'_> {
    pub(super) fn enumeration(&self, name: &str) -> Option<&Enum> {
        self.enums.iter().find(|e| e.name == name)
    }

    pub(super) fn record(&self, name: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.name == name)
    }

    /// The types `ty`'s values are made of, one level down: a list's
    /// element, a record's fields, each variant's fields.
    fn parts(&self, ty: &Type) -> Vec<Type> {
        match ty {
            Type::List(element) => vec![(**element).clone()],
            Type::Record(name) => self
                .record(name)
                .map(|r| r.fields.iter().map(|(_, t)| t.clone()).collect())
                .unwrap_or_default(),
            Type::Enum(name) => self
                .enumeration(name)
                .map(|e| e.fields.iter().flatten().map(|(_, t)| t.clone()).collect())
                .unwrap_or_default(),
            Type::Bool | Type::Int(_) | Type::Text => Vec::new(),
        }
    }

    /// Whether `ty`'s values hold, anywhere inside, a value that `holds`
    /// accepts; `ty` itself included.
    fn holds(&self, ty: &Type, holds: &impl Fn(&Type) -> bool) -> bool {
        let mut pending = vec![ty.clone()];
        let mut seen: Vec<Type> = Vec::new();
        while let Some(ty) = pending.pop() {
            if holds(&ty) {
                return true;
            }
            if !seen.contains(&ty) {
                pending.extend(self.parts(&ty));
                seen.push(ty);
            }
        }
        false
    }

    /// Whether `ty` is or holds a record or a variant with fields.
    pub(super) fn structured(&self, ty: &Type) -> bool {
        self.holds(ty, &|t| match t {
            Type::Record(_) => true,
            Type::Enum(name) => self.enumeration(name).is_some_and(Enum::has_fields),
            _ => false,
        })
    }
}

/// `ty` with each name of a record read as the record's type: the parser
/// reads every name it doesn't know as an enum's.
fn resolve(ty: &mut Type, records: &[Record]) {
    match ty {
        Type::Enum(name) if records.iter().any(|r| r.name == *name) => {
            *ty = Type::Record(std::mem::take(name));
        }
        Type::List(element) => resolve(element, records),
        _ => {}
    }
}

/// Reads every record's name in `program`'s declared types as the record's.
fn resolve_records(program: &mut Program) {
    let records = program.records.clone();
    let mut slots: Vec<&mut Type> = Vec::new();
    for e in &mut program.enums {
        slots.extend(e.fields.iter_mut().flatten().map(|(_, t)| t));
    }
    for r in &mut program.records {
        slots.extend(r.fields.iter_mut().map(|(_, t)| t));
    }
    let mut functions: Vec<&mut Function> = program.functions.iter_mut().collect();
    for m in &mut program.machines {
        slots.extend(m.state.iter_mut().map(|(_, t)| t));
        functions.extend(
            m.inits
                .iter_mut()
                .chain(&mut m.events)
                .chain(&mut m.observers),
        );
    }
    for f in functions {
        slots.extend(f.params.iter_mut().map(|(_, t)| t));
        slots.push(&mut f.ret);
    }
    for ty in slots {
        resolve(ty, &records);
    }
}

/// A record's or a variant's fields, each with its name and type.
type Fields = [(String, Type)];

/// Every record's and enum's name once, each field's name once within its
/// record or variant, each field's type known, and no type that holds
/// itself.
fn check_declared(types: Types) -> Result<(), Diagnostic> {
    let declared: Vec<(&String, usize)> = types
        .enums
        .iter()
        .map(|e| (&e.name, e.line))
        .chain(types.records.iter().map(|r| (&r.name, r.line)))
        .collect();
    for (index, (name, line)) in declared.iter().enumerate() {
        if *name == "Bool" || declared[..index].iter().any(|(other, _)| other == name) {
            return Err(Diagnostic::new(
                *line,
                format!("the type `{name}` is already defined"),
            ));
        }
    }
    let mut groups: Vec<(&String, &Fields, usize)> = Vec::new();
    for e in types.enums {
        for (variant, fields) in e.variants.iter().zip(&e.fields) {
            groups.push((variant, fields, e.line));
        }
    }
    for r in types.records {
        groups.push((&r.name, &r.fields, r.line));
    }
    for (owner, fields, line) in groups {
        for (index, (field, ty)) in fields.iter().enumerate() {
            if fields[..index].iter().any(|(other, _)| other == field) {
                return Err(Diagnostic::new(
                    line,
                    format!("`{owner}` has the field `{field}` twice"),
                ));
            }
            check_type(types, ty, line)?;
        }
    }
    for (name, line) in declared {
        let own = Type::Enum(name.clone());
        let ty = if types.record(name).is_some() {
            Type::Record(name.clone())
        } else {
            own
        };
        let inside = types.parts(&ty);
        if inside.iter().any(|part| types.holds(part, &|t| *t == ty)) {
            return Err(Diagnostic::new(
                line,
                format!("`{name}` holds a `{name}`: a type can't contain itself yet"),
            ));
        }
    }
    Ok(())
}

fn check_enums(enums: &[Enum]) -> Result<(), Diagnostic> {
    for e in enums {
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

/// A list's elements hold no integer and no list, anywhere inside: a script
/// can't yet assume each element's integers are in range and its lists
/// within their length.
fn check_type(types: Types, ty: &Type, line: usize) -> Result<(), Diagnostic> {
    match ty {
        Type::List(element) => {
            check_type(types, element, line)?;
            if types.holds(element, &|t| matches!(t, Type::Int(_) | Type::List(_))) {
                return Err(Diagnostic::new(
                    line,
                    format!("lists of `{}` aren't supported yet", type_name(element)),
                ));
            }
            Ok(())
        }
        Type::Enum(name) if types.enumeration(name).is_none() => {
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
    let type_file = |line: usize, p: &Program| {
        p.enums
            .iter()
            .map(|e| (e.line, e.file))
            .chain(p.records.iter().map(|r| (r.line, r.file)))
            .find(|(l, _)| *l == line)
            .map_or(0, |(_, file)| file)
    };
    resolve_records(program);
    let types = Types {
        enums: &program.enums,
        records: &program.records,
    };
    check_declared(types)
        .and_then(|()| check_enums(&program.enums))
        .map_err(|d| {
            let file = type_file(d.line, program);
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
    let types = Types {
        enums: &program.enums,
        records: &program.records,
    };
    let signatures: Vec<Signature> = program
        .functions
        .iter()
        .map(|f| Signature {
            name: f.name.clone(),
            params: f.params.clone(),
            ret: f.ret.clone(),
            state: Vec::new(),
        })
        .collect();
    for function in &mut program.functions {
        let file = name(function.file);
        check_function(types, &signatures, function).map_err(|d| d.in_file(&file))?;
    }
    check_recursion(program).map_err(|d| {
        let file = program
            .functions
            .iter()
            .find(|f| f.line == d.line)
            .map_or(0, |f| f.file);
        d.in_file(&name(file))
    })?;
    for (index, machine) in program.machines.iter().enumerate() {
        if program.machines[..index]
            .iter()
            .any(|other| other.name == machine.name)
            || program.functions.iter().any(|f| f.name == machine.name)
        {
            return Err(Diagnostic::new(
                machine.line,
                format!("`{}` is declared more than once", machine.name),
            )
            .in_file(&name(machine.file)));
        }
    }
    for machine in &mut program.machines {
        let file = name(machine.file);
        machine::check(types, &signatures, &program.functions, machine)
            .map_err(|d| d.in_file(&file))?;
    }
    let functions = &program.functions;
    for example in &mut program.examples {
        let file = name(example.file);
        example::check(types, functions, example).map_err(|d| d.in_file(&file))?;
    }
    for run in &mut program.machine_examples {
        let file = name(run.file);
        machine::check_run(types, &program.machines, run).map_err(|d| d.in_file(&file))?;
    }
    Ok(())
}

/// Checks each clause of `function` against its parameters.
fn check_clauses(types: Types, function: &mut Function) -> Result<(), Diagnostic> {
    for clause in &mut function.clauses {
        let scope = Scope {
            types,
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
    Ok(())
}

fn check_function(
    types: Types,
    signatures: &[Signature],
    function: &mut Function,
) -> Result<(), Diagnostic> {
    for (_, ty) in &function.params {
        check_type(types, ty, function.line)?;
    }
    check_type(types, &function.ret, function.line)?;
    check_clauses(types, function)?;
    let scope = Scope {
        types,
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
