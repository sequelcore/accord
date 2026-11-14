//! A checked program as TypeScript modules: one per Accord file, with the
//! same base name. Each `pub` function becomes an exported function that
//! checks what the proofs assume of its caller, its parameters' types and
//! ranges and its `requires`, and then calls the function's body, written as
//! an unexported function whose name ends in `$`. Every enum becomes an
//! exported union of string values.

mod boundary;
#[cfg(test)]
mod differential;
mod expr;
mod lists;

use std::collections::BTreeMap;

use crate::openness::{Determinacy, Openness};
use crate::syntax::{Body, Enum, Expr, Function, Machine, Program, Role, Type, Width};
use crate::{Diagnostic, Outcome, Report, Run, Solvers};
use boundary::{TEXT_CHECK, state_check, state_type, wrapper};
use expr::{Context, Scope, camel, string};

const HEADER: &str = "// Every obligation of these functions is proved. This TypeScript is not
// proved equivalent to them: Accord tests its code generator against its
// interpreter.
// Each exported function checks its parameters and its `requires`, and
// throws a TypeError or a RangeError when one doesn't hold.
";

const MACHINE_HEADER: &str =
    "// A machine's events and observers also check that the state they are given
// has its fields and meets every invariant and helper invariant. An event
// returns the next state and leaves the one it is given unchanged.
";

/// One generated TypeScript module.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    /// The module's file name: its Accord file's base name with `.ts`.
    pub name: String,
    /// Its source.
    pub source: String,
}

/// What `build` established, and the modules when it emitted them.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    /// The check of the whole program.
    pub report: Report,
    /// One module per file, in the files' order; `None` when an obligation
    /// wasn't proved or an example didn't pass, so nothing may be emitted.
    pub modules: Option<Vec<Module>>,
}

/// Whether every obligation is proved and every example passed.
fn proved(report: &Report) -> bool {
    report
        .verdicts
        .iter()
        .all(|v| matches!(v.outcome, Outcome::Proved { .. }))
        && report.examples.iter().all(|e| e.run == Run::Passed)
}

/// Checks a program of one or more files, each a file's name and its source,
/// and writes it as TypeScript when every obligation is proved and every
/// example passes. An open case doesn't block it: each module's header
/// names the functions that have one.
pub fn build(files: &[(String, String)], solvers: &Solvers) -> Result<Build, Diagnostic> {
    let (program, names) = crate::program_of(files)?;
    check_writable(&program, &names)?;
    check_exports(&program, &names)?;
    let report = crate::report(&program, None, &names, solvers);
    let modules = proved(&report).then(|| {
        modules(&program, &names, &report.openness)
            .into_iter()
            .map(|(name, source)| Module { name, source })
            .collect()
    });
    Ok(Build { report, modules })
}

/// Rejects a program with a record, a variant with fields or a `match`,
/// which aren't written as TypeScript yet.
fn check_writable(program: &Program, names: &[String]) -> Result<(), Diagnostic> {
    let refuse = |what: &str, line: usize, file: usize| {
        Err(Diagnostic::new(
            line,
            format!("`accord build` doesn't write {what} as TypeScript yet"),
        )
        .in_file(names.get(file).map_or("", String::as_str)))
    };
    if let Some(r) = program.records.first() {
        return refuse("records", r.line, r.file);
    }
    if let Some(e) = program.enums.iter().find(|e| e.has_fields()) {
        return refuse("variants with fields", e.line, e.file);
    }
    fn has_match(expr: &Expr) -> bool {
        matches!(expr, Expr::Match(..)) || expr.children().into_iter().any(has_match)
    }
    let members = program.machines.iter().flat_map(Machine::members);
    for function in program.functions.iter().chain(members) {
        let mut exprs: Vec<&Expr> = function.clauses.iter().map(|c| &c.expr).collect();
        match &function.body {
            Body::Expr(body) => exprs.push(body),
            Body::Decide(rows, _) => {
                for row in rows {
                    exprs.extend(row.condition.as_ref());
                    exprs.push(&row.result);
                }
            }
        }
        if exprs.into_iter().any(has_match) {
            return refuse("`match`", function.line, function.file);
        }
    }
    Ok(())
}

/// The public machines declared in `file`.
fn machines(program: &Program, file: usize) -> Vec<&Machine> {
    program
        .machines
        .iter()
        .filter(|m| m.file == file && m.public)
        .collect()
}

/// Rejects a file whose module would declare one name twice: an exported
/// function, a function's body or a machine's check, whichever it comes from.
fn check_exports(program: &Program, names: &[String]) -> Result<(), Diagnostic> {
    for (file, name) in names.iter().enumerate() {
        let functions: Vec<&Function> = program
            .functions
            .iter()
            .filter(|f| f.file == file && f.public)
            .collect();
        let mut declared: Vec<(String, usize)> = Vec::new();
        let mut members: Vec<&Function> = Vec::new();
        for function in &functions {
            let exported = camel(&function.name);
            declared.push((format!("{exported}$"), function.line));
            declared.push((exported, function.line));
        }
        for e in program.enums.iter().filter(|e| e.file == file) {
            declared.push((format!("type {}", e.name), e.line));
        }
        for machine in machines(program, file) {
            declared.push((format!("type {}", machine.name), machine.line));
            declared.push((format!("check{}$", machine.name), machine.line));
            for member in machine.members() {
                let exported = camel(&member.name);
                declared.push((format!("{exported}$"), member.line));
                declared.push((exported, member.line));
                members.push(member);
            }
        }
        let all: Vec<&Function> = functions.iter().chain(&members).copied().collect();
        declared.extend(
            called(program, &all)
                .iter()
                .map(|f| (format!("{}$", camel(&f.name)), f.line)),
        );
        for (index, (declaration, line)) in declared.iter().enumerate() {
            if declared[..index]
                .iter()
                .any(|(other, _)| other == declaration)
            {
                return Err(Diagnostic::new(
                    *line,
                    format!("`{declaration}` is declared twice in the module this file becomes"),
                )
                .in_file(name));
            }
        }
    }
    Ok(())
}

/// The modules for `program`'s files, named after `names`, in their order.
fn modules(program: &Program, names: &[String], openness: &[Determinacy]) -> Vec<(String, String)> {
    names
        .iter()
        .enumerate()
        .map(|(file, name)| {
            let stem = name.strip_suffix(".accord").unwrap_or(name);
            (format!("{stem}.ts"), module(program, names, file, openness))
        })
        .collect()
}

fn module(program: &Program, names: &[String], file: usize, openness: &[Determinacy]) -> String {
    let functions: Vec<&Function> = program
        .functions
        .iter()
        .filter(|f| f.file == file && f.public)
        .collect();
    let machines = machines(program, file);
    let members: Vec<&Function> = machines.iter().flat_map(|m| m.members()).collect();
    let mut out = format!(
        "// Generated by accord from {}. Do not edit.\n{HEADER}",
        names[file]
    );
    if !machines.is_empty() {
        out += MACHINE_HEADER;
    }
    for function in &functions {
        let found = openness.iter().find(|d| d.function == function.name);
        match found.map(|d| &d.openness) {
            Some(Openness::Open { .. }) => {
                out += &format!(
                    "// Open case in {}: its contract allows more than one result for some inputs.\n",
                    camel(&function.name)
                )
            }
            Some(Openness::Undecided { .. }) => {
                out += &format!(
                    "// Open-case search undecided in {}: whether its contract allows more than one result wasn't settled.\n",
                    camel(&function.name)
                )
            }
            _ => {}
        }
    }
    let mut items = Vec::new();
    let everything: Vec<&Function> = functions.iter().chain(&members).copied().collect();
    let imports = imports(program, names, file, &everything);
    if !imports.is_empty() {
        items.push(imports);
    }
    for e in program.enums.iter().filter(|e| e.file == file) {
        let values: Vec<String> = e.variants.iter().map(|v| string(&e.text(v))).collect();
        items.push(format!(
            "export type {} = {};\n",
            e.name,
            values.join(" | ")
        ));
    }
    let mut text = false;
    for function in &functions {
        let (wrapper, uses_text) = wrapper(program, function, None);
        text |= uses_text;
        items.push(wrapper);
        items.push(core(function, program, &program.functions));
    }
    for machine in &machines {
        items.push(state_type(machine));
        let (check, uses_text) = state_check(program, machine);
        text |= uses_text;
        items.push(check);
        let callable = machine.functions(&program.functions);
        for member in machine.members() {
            let (wrapper, uses_text) = wrapper(program, member, Some(machine));
            text |= uses_text;
            items.push(wrapper);
            items.push(match member.machine.map(|m| m.role) {
                Some(Role::Observer) => core(member, program, &callable),
                _ => transition_core(member, machine, program, &callable),
            });
        }
    }
    // A called function's body is written into every module that calls it,
    // unexported, so no module exports an unchecked entry point.
    for function in called(program, &everything) {
        items.push(core(function, program, &program.functions));
    }
    if text {
        items.push(TEXT_CHECK.to_string());
    }
    for item in items {
        out += "\n";
        out += &item;
    }
    out
}

/// Every function `functions` call, directly or not, that isn't among them,
/// in program order.
fn called<'a>(program: &'a Program, functions: &[&Function]) -> Vec<&'a Function> {
    let mut names: Vec<String> = Vec::new();
    let mut pending: Vec<String> = functions
        .iter()
        .flat_map(|f| crate::check::body_callees(f))
        .collect();
    while let Some(name) = pending.pop() {
        if names.contains(&name) || functions.iter().any(|f| f.name == name) {
            continue;
        }
        if let Some(callee) = program.functions.iter().find(|f| f.name == name) {
            pending.extend(crate::check::body_callees(callee));
        }
        names.push(name);
    }
    program
        .functions
        .iter()
        .filter(|f| names.contains(&f.name))
        .collect()
}

/// `import type` lines for the enums `functions`' signatures use that
/// another file declares.
fn imports(program: &Program, names: &[String], file: usize, functions: &[&Function]) -> String {
    let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for function in functions {
        let types = function
            .params
            .iter()
            .map(|(_, ty)| ty)
            .chain([&function.ret]);
        for ty in types {
            let name = match ty {
                Type::Enum(name) => name,
                Type::List(element) => match element.as_ref() {
                    Type::Enum(name) => name,
                    _ => continue,
                },
                _ => continue,
            };
            let Some(e) = enumeration(program, name) else {
                continue;
            };
            if e.file != file {
                let stem = names[e.file]
                    .strip_suffix(".accord")
                    .unwrap_or(&names[e.file]);
                let entry = by_file.entry(stem).or_default();
                if !entry.contains(&e.name.as_str()) {
                    entry.push(&e.name);
                }
            }
        }
    }
    let mut out = String::new();
    for (stem, mut types) in by_file {
        types.sort_unstable();
        out += &format!(
            "import type {{ {} }} from \"./{stem}.js\";\n",
            types.join(", ")
        );
    }
    out
}

fn enumeration<'a>(program: &'a Program, name: &str) -> Option<&'a Enum> {
    program.enums.iter().find(|e| e.name == name)
}

/// A type as the exported function's caller sees it.
fn outer(ty: &Type) -> String {
    match ty {
        Type::Bool => "boolean".to_string(),
        Type::Int(_) => "number".to_string(),
        Type::Text => "string".to_string(),
        Type::List(element) => format!("readonly {}[]", outer(element)),
        Type::Enum(name) | Type::Record(name) => name.clone(),
    }
}

/// A type as the body sees it: `i64` is a `bigint`.
fn inner(ty: &Type) -> String {
    match ty {
        Type::Int(Width::I64) => "bigint".to_string(),
        other => outer(other),
    }
}

fn signature(function: &Function, name: &str, ty: fn(&Type) -> String) -> String {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|(n, t)| format!("{}: {}", camel(n), ty(t)))
        .collect();
    format!("{name}({}): {}", params.join(", "), ty(&function.ret))
}

/// The body of an init or event, unexported: it takes the state's fields,
/// for an event, then its own parameters, and returns the next state.
fn transition_core(
    function: &Function,
    machine: &Machine,
    program: &Program,
    functions: &[Function],
) -> String {
    let skip = match function.machine {
        Some(member) if member.role == Role::Init => member.state,
        _ => 0,
    };
    let params: Vec<String> = function.params[skip..]
        .iter()
        .map(|(n, t)| format!("{}: {}", camel(n), inner(t)))
        .collect();
    let code = Scope {
        params: &function.params,
        context: Context::Code,
        functions,
        enums: &program.enums,
        indices: &[],
    };
    let mut out = format!(
        "function {}$({}): {} {{\n",
        camel(&function.name),
        params.join(", "),
        machine.name
    );
    match &function.body {
        Body::Expr(body) => out += &transition(&code, machine, body, "  "),
        Body::Decide(rows, _) => {
            for row in rows {
                out += &match &row.condition {
                    Some(condition) => {
                        let condition = code.bare(condition, None);
                        branch(&code, machine, &condition, &row.result, "  ")
                    }
                    None => transition(&code, machine, &row.result, "  "),
                };
            }
            if rows.last().is_some_and(|row| row.condition.is_some()) {
                out += &format!(
                    "  throw new Error({});\n",
                    string(&format!("{}: no row matched", camel(&function.name)))
                );
            }
        }
    }
    out + "}\n"
}

/// The statements that return the state an `if`, or a `next`, ends in.
fn transition(code: &Scope, machine: &Machine, expr: &Expr, indent: &str) -> String {
    match expr {
        Expr::If(condition, then, otherwise) => {
            let condition = code.bare(condition, None);
            branch(code, machine, &condition, then, indent)
                + &transition(code, machine, otherwise, indent)
        }
        Expr::Next(fields, _) => {
            format!("{indent}return {};\n", state_object(code, machine, fields))
        }
        _ => unreachable!("`check` ends every init and event in a `next`"),
    }
}

/// `if (condition)` and what returns the state `then` ends in.
fn branch(code: &Scope, machine: &Machine, condition: &str, then: &Expr, indent: &str) -> String {
    match then {
        Expr::Next(fields, _) => format!(
            "{indent}if ({condition}) return {};\n",
            state_object(code, machine, fields)
        ),
        _ => format!(
            "{indent}if ({condition}) {{\n{}{indent}}}\n",
            transition(code, machine, then, &format!("{indent}  "))
        ),
    }
}

/// The state as an object literal, every field in the state's order.
fn state_object(code: &Scope, machine: &Machine, fields: &[(String, Expr)]) -> String {
    let parts: Vec<String> = fields
        .iter()
        .zip(&machine.state)
        .map(|((_, value), (field, ty))| {
            let want = match ty {
                Type::Int(width) => Some(*width),
                _ => None,
            };
            let value = code.bare(value, want);
            let value = if *ty == Type::Int(Width::I64) {
                format!("Number({value})")
            } else {
                value
            };
            format!("{}: {value}", camel(field))
        })
        .collect();
    format!("{{ {} }}", parts.join(", "))
}

/// The function's body, unexported; it may call `functions`.
fn core(function: &Function, program: &Program, functions: &[Function]) -> String {
    let name = format!("{}$", camel(&function.name));
    let code = Scope {
        params: &function.params,
        context: Context::Code,
        functions,
        enums: &program.enums,
        indices: &[],
    };
    let want = match function.ret {
        Type::Int(width) => Some(width),
        _ => None,
    };
    let mut out = format!("function {} {{\n", signature(function, &name, inner));
    match &function.body {
        Body::Expr(body) => out += &format!("  return {};\n", code.bare(body, want)),
        Body::Decide(rows, _) => {
            for row in rows {
                let result = code.bare(&row.result, want);
                out += &match &row.condition {
                    Some(condition) => {
                        format!("  if ({}) return {result};\n", code.bare(condition, None))
                    }
                    None => format!("  return {result};\n"),
                };
            }
            if rows.last().is_some_and(|row| row.condition.is_some()) {
                out += &format!(
                    "  throw new Error({});\n",
                    string(&format!("{}: no row matched", camel(&function.name)))
                );
            }
        }
    }
    out + "}\n"
}
