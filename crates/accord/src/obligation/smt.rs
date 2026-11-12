//! Writing SMT-LIB: terms, sorts and ranges, a function's prelude, and the
//! ending every script shares.

use crate::syntax::{Arith, Body, ClauseKind, Expr, Function, Op, Program, Row, Type, Width};

pub(crate) fn sort(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int(_) => "Int".to_string(),
        Type::Text => "String".to_string(),
        Type::List(element) => format!("(Seq {})", sort(element)),
        Type::Enum(name) => name.clone(),
    }
}
pub(crate) fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::Int(value, _) => number(*value),
        Expr::Text(text, _) => string(text),
        Expr::List(items, element, _) => {
            let units: Vec<String> = items
                .iter()
                .map(|i| format!("(seq.unit {})", term(i)))
                .collect();
            match units.len() {
                0 => format!(
                    "(as seq.empty (Seq {}))",
                    element.as_ref().map_or("Bool".to_string(), sort)
                ),
                1 => units.concat(),
                _ => format!("(seq.++ {})", units.join(" ")),
            }
        }
        Expr::Name(name, _) => name.clone(),
        Expr::Variant(owner, name, _) => format!("{owner}.{name}"),
        Expr::Not(inner) => format!("(not {})", term(inner)),
        Expr::Binary(op, left, right) => {
            let (l, r) = (term(left), term(right));
            match op {
                Op::And => format!("(and {l} {r})"),
                Op::Or => format!("(or {l} {r})"),
                Op::Implies => format!("(=> {l} {r})"),
                Op::Eq => format!("(= {l} {r})"),
                Op::Ne => format!("(not (= {l} {r}))"),
                Op::Lt => format!("(< {l} {r})"),
                Op::Le => format!("(<= {l} {r})"),
                Op::Gt => format!("(> {l} {r})"),
                Op::Ge => format!("(>= {l} {r})"),
                Op::In => format!("(seq.contains {r} (seq.unit {l}))"),
            }
        }
        Expr::Arith(op, left, right, _) => {
            let symbol = match op {
                Arith::Add => "+",
                Arith::Sub => "-",
                Arith::Mul => "*",
            };
            format!("({symbol} {} {})", term(left), term(right))
        }
        Expr::Neg(inner, _) => format!("(- {})", term(inner)),
        Expr::If(condition, then, otherwise) => {
            format!(
                "(ite {} {} {})",
                term(condition),
                term(then),
                term(otherwise)
            )
        }
        Expr::Call(name, args, _) => apply(name, args.iter().map(|(_, a)| term(a)).collect()),
    }
}
/// `name` applied to `args`; a function of no arguments is a constant.
fn apply(name: &str, args: Vec<String>) -> String {
    if args.is_empty() {
        name.to_string()
    } else {
        format!("({name} {})", args.join(" "))
    }
}
/// The callee a checked call names, with its arguments' terms.
fn callee<'a>(program: &'a Program, call: &Expr) -> Option<(&'a Function, Vec<String>)> {
    let Expr::Call(name, args, _) = call else {
        return None;
    };
    let function = program.functions.iter().find(|f| f.name == *name)?;
    Some((function, args.iter().map(|(_, a)| term(a)).collect()))
}
/// Whether the function a call names has a `requires`.
pub(crate) fn has_requires(program: &Program, call: &Expr) -> bool {
    callee(program, call)
        .is_some_and(|(f, _)| f.clauses.iter().any(|c| c.kind == ClauseKind::Requires))
}
/// That the callee's `requires` holds of a call's arguments.
pub(crate) fn requires_of(program: &Program, call: &Expr) -> String {
    match callee(program, call) {
        Some((f, args)) => apply(&format!("{}.requires", f.name), args),
        None => "true".to_string(),
    }
}
/// The SMT-LIB declarations of the functions `names`: each one's `requires`
/// and `ensures` as predicates, and the function itself, of which a script
/// knows only what its contract says.
pub(crate) fn declarations(program: &Program, names: &[String]) -> String {
    let mut out = String::new();
    for name in names {
        let Some(f) = program.functions.iter().find(|f| f.name == *name) else {
            continue;
        };
        let params: Vec<String> = f
            .params
            .iter()
            .map(|(n, ty)| format!("({n} {})", sort(ty)))
            .collect();
        let clauses = |kind: ClauseKind| {
            all(
                "and",
                f.clauses
                    .iter()
                    .filter(|c| c.kind == kind)
                    .map(|c| term(&c.expr))
                    .collect(),
                "true",
            )
        };
        let ret = sort(&f.ret);
        out += &format!(
            "(define-fun {name}.requires ({}) Bool {})\n",
            params.join(" "),
            clauses(ClauseKind::Requires)
        );
        let mut with_result = params.clone();
        with_result.push(format!("(result {ret})"));
        out += &format!(
            "(define-fun {name}.ensures ({}) Bool {})\n",
            with_result.join(" "),
            clauses(ClauseKind::Ensures)
        );
        let sorts: Vec<String> = f.params.iter().map(|(_, ty)| sort(ty)).collect();
        out += &format!("(declare-fun {name} ({}) {ret})\n", sorts.join(" "));
    }
    out
}
/// What a script knows of each distinct call in `calls`: a constant, named
/// in the returned list, equal to the call's value, and, where the callee's
/// `requires` holds of its arguments, its `ensures` and its result type's
/// range.
pub(crate) fn call_facts(program: &Program, calls: &[&Expr]) -> (String, Vec<String>) {
    let (mut out, mut names, mut seen) = (String::new(), Vec::new(), Vec::<String>::new());
    for call in calls {
        let Some((f, args)) = callee(program, call) else {
            continue;
        };
        let value = term(call);
        if seen.contains(&value) {
            continue;
        }
        let constant = format!("c!{}", names.len());
        let mut holds = vec![apply(&format!("{}.ensures", f.name), {
            let mut with_result = args.clone();
            with_result.push(value.clone());
            with_result
        })];
        if let Type::Int(width) = f.ret {
            holds.push(in_range(&value, width));
        }
        out += &format!(
            "(declare-const {constant} {})\n(assert (= {constant} {value}))\n(assert (=> {} {}))\n",
            sort(&f.ret),
            apply(&format!("{}.requires", f.name), args),
            all("and", holds, "true")
        );
        seen.push(value);
        names.push(constant);
    }
    (out, names)
}
/// A text as an SMT-LIB string literal. SMT-LIB reads `\u{...}` in a
/// literal as an escape, so a backslash, and every character outside
/// printable ASCII, is written as one; the text means the same to the
/// solvers as to the program.
fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\"\""),
            ' '..='~' if c != '\\' => out.push(c),
            _ => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
        }
    }
    out.push('"');
    out
}

fn number(value: i128) -> String {
    if value < 0 {
        format!("(- {})", value.unsigned_abs())
    } else {
        value.to_string()
    }
}
/// That `term` is a value of `width`.
pub(crate) fn in_range(term: &str, width: Width) -> String {
    format!(
        "(and (<= {} {term}) (<= {term} {}))",
        number(width.min()),
        number(width.max())
    )
}
fn condition(row: &Row) -> String {
    row.condition.as_ref().map_or("true".to_string(), term)
}
/// `and` or `or` over any number of terms; SMT-LIB needs two or more.
pub(crate) fn all(op: &str, terms: Vec<String>, empty: &str) -> String {
    match terms.len() {
        0 => empty.to_string(),
        1 => terms.concat(),
        _ => format!("({op} {})", terms.join(" ")),
    }
}
/// The value of a decision table: the first row whose condition holds. Past
/// the last row the value is the last row's; coverage is its own obligation.
fn decide_term(rows: &[Row]) -> String {
    let Some((last, earlier)) = rows.split_last() else {
        return String::new();
    };
    earlier.iter().rev().fold(term(&last.result), |rest, row| {
        format!("(ite {} {} {rest})", condition(row), term(&row.result))
    })
}
/// What every script about `function` starts with: the enums it uses, its
/// parameters within their ranges, and its `requires` assumed.
pub(crate) fn prelude(function: &Function, datatypes: &str) -> String {
    let mut prelude = format!("(set-option :produce-models true)\n(set-logic ALL)\n{datatypes}");
    for (name, ty) in &function.params {
        prelude += &format!("(declare-const {name} {})\n", sort(ty));
        if let Type::Int(width) = ty {
            prelude += &format!("(assert {})\n", in_range(name, *width));
        }
    }
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        prelude += &format!("(assert {})\n", term(&clause.expr));
    }
    prelude
}
/// A script's ending: check, then the values of `names`, then why an
/// `unknown` was unknown. Asked last, so its error after `sat` never precedes
/// the values.
pub(crate) fn ending(names: &[String]) -> String {
    let values = if names.is_empty() {
        String::new()
    } else {
        format!("(get-value ({}))\n", names.join(" "))
    };
    format!("(check-sat)\n{values}(get-info :reason-unknown)\n")
}
/// A body's value as a term, and for a decision table the condition that some
/// row matches.
pub(crate) fn value(function: &Function) -> (String, Option<String>) {
    match &function.body {
        Body::Expr(body) => (term(body), None),
        Body::Decide(rows, _) => (
            decide_term(rows),
            Some(all("or", rows.iter().map(condition).collect(), "false")),
        ),
    }
}
/// The SMT-LIB declarations of a program's enums.
pub(crate) fn datatypes(program: &Program) -> String {
    program
        .enums
        .iter()
        .map(|e| {
            let constructors: Vec<String> = e
                .variants
                .iter()
                .map(|v| format!("({}.{v})", e.name))
                .collect();
            format!(
                "(declare-datatype {} ({}))\n",
                e.name,
                constructors.join(" ")
            )
        })
        .collect()
}
