//! Turns each `ensures` clause into an SMT-LIB script whose satisfiability
//! means the clause can fail.

use crate::syntax::{ClauseKind, Expr, Function, Op, Type};

/// One logical statement that must hold, written as an SMT-LIB script.
pub(crate) struct Obligation {
    pub function: String,
    pub clause: String,
    pub line: usize,
    pub params: Vec<String>,
    pub script: String,
}

fn sort(ty: &Type) -> &'static str {
    match ty {
        Type::Bool => "Bool",
    }
}

fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::Name(name, _) => name.clone(),
        Expr::Not(inner) => format!("(not {})", term(inner)),
        Expr::Binary(op, left, right) => {
            let (l, r) = (term(left), term(right));
            match op {
                Op::And => format!("(and {l} {r})"),
                Op::Or => format!("(or {l} {r})"),
                Op::Implies => format!("(=> {l} {r})"),
                Op::Eq => format!("(= {l} {r})"),
                Op::Ne => format!("(not (= {l} {r}))"),
            }
        }
        Expr::If(condition, then, otherwise) => {
            format!(
                "(ite {} {} {})",
                term(condition),
                term(then),
                term(otherwise)
            )
        }
    }
}

/// The obligations of every function: one per `ensures`, in source order.
///
/// Each script declares the parameters and `result`, assumes every
/// `requires` and that `result` is the body's value, and asserts that the
/// clause fails. `unsat` means the clause holds for every admitted input.
pub(crate) fn obligations(functions: &[Function]) -> Vec<Obligation> {
    let mut out = Vec::new();
    for function in functions {
        let mut prelude = String::from("(set-option :produce-models true)\n(set-logic QF_UF)\n");
        for (name, ty) in &function.params {
            prelude += &format!("(declare-const {name} {})\n", sort(ty));
        }
        prelude += &format!("(declare-const result {})\n", sort(&function.ret));
        for clause in function
            .clauses
            .iter()
            .filter(|c| c.kind == ClauseKind::Requires)
        {
            prelude += &format!("(assert {})\n", term(&clause.expr));
        }
        prelude += &format!("(assert (= result {}))\n", term(&function.body));
        let params: Vec<String> = function
            .params
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let values = if params.is_empty() {
            String::new()
        } else {
            format!("(get-value ({}))\n", params.join(" "))
        };
        for clause in function
            .clauses
            .iter()
            .filter(|c| c.kind == ClauseKind::Ensures)
        {
            out.push(Obligation {
                function: function.name.clone(),
                clause: clause.text.clone(),
                line: clause.line,
                params: params.clone(),
                script: format!(
                    "{prelude}(assert (not {}))\n(check-sat)\n{values}",
                    term(&clause.expr)
                ),
            });
        }
    }
    out
}
