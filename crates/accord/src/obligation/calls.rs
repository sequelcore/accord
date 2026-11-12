//! Calls in SMT-LIB: each callee is a function a script knows only through
//! its contract.

use super::operations::{Operation, for_every};
use super::smt::{all, in_range, sort, term};
use crate::syntax::{ClauseKind, Expr, Function, Program, Type};

/// `name` applied to `args`; a function of no arguments is a constant.
pub(super) fn apply(name: &str, args: Vec<String>) -> String {
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
/// What a script knows of each distinct call in `calls`: where the callee's
/// `requires` holds of its arguments, its `ensures` and its result type's
/// range. A call outside any quantifier also has a constant equal to its
/// value, returned with the call; inside one, the facts hold for every index
/// in range, and its value at each has no constant.
pub(crate) fn call_facts<'a>(
    program: &Program,
    calls: &[Operation<'a>],
) -> (String, Vec<(String, &'a Expr)>) {
    let (mut out, mut names, mut seen) = (String::new(), Vec::new(), Vec::<String>::new());
    for op in calls {
        let call = op.expr;
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
        let fact = format!(
            "(=> {} {})",
            apply(&format!("{}.requires", f.name), args),
            all("and", holds, "true")
        );
        if op.bound.is_empty() {
            out += &format!(
                "(declare-const {constant} {})\n(assert (= {constant} {value}))\n(assert {fact})\n",
                sort(&f.ret)
            );
            names.push((constant, call));
        } else {
            out += &format!("(assert {})\n", for_every(&op.bound, fact));
        }
        seen.push(value);
    }
    (out, names)
}
