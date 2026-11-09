//! Turns a checked program into SMT-LIB scripts, one per obligation.
//!
//! A universal obligation must hold for every admitted input: its script
//! asserts that it fails, so `unsat` proves it. An existential obligation
//! needs some admitted input, such as one that reaches a decision table's
//! row: its script asserts that the input exists, so `sat` proves it.
//!
//! Integers are mathematical in every script. A parameter of a fixed width is
//! assumed to be in its range, and each arithmetic operation in code that
//! runs carries the obligation that its result stays in range, so the
//! mathematical value is the one the program computes.

use crate::syntax::{Arith, Body, ClauseKind, Expr, Function, Op, Program, Row, Site, Type, Width};

/// Whether an obligation is about every input or about some input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Polarity {
    Universal,
    Existential,
}

/// One logical statement that must hold, written as an SMT-LIB script, with
/// what it claims in source terms so a solver's model can be replayed.
pub(crate) struct Obligation<'a> {
    pub function: &'a Function,
    pub clause: String,
    pub line: usize,
    pub params: Vec<String>,
    pub polarity: Polarity,
    pub claim: Claim<'a>,
    pub script: String,
}

/// What an obligation states, for admitted inputs (every `requires` holds).
pub(crate) enum Claim<'a> {
    /// The `ensures` holds of the body's value.
    Ensures(&'a Expr),
    /// Some row of the decision table matches.
    Covered,
    /// The row at this index is the first that matches.
    Reaches(usize),
    /// Where the guards hold, `expr` stays within `width`.
    InRange {
        expr: &'a Expr,
        width: Width,
        guards: Vec<Guard<'a>>,
    },
}

/// A condition that must have a given value for code to run; `None` is a
/// row's `otherwise`, which always holds.
#[derive(Clone, Copy)]
pub(crate) struct Guard<'a> {
    pub condition: Option<&'a Expr>,
    pub holds: bool,
}

fn guard_term(guard: &Guard) -> String {
    let c = guard.condition.map_or("true".to_string(), term);
    if guard.holds { c } else { format!("(not {c})") }
}

pub(crate) fn sort(ty: &Type) -> &str {
    match ty {
        Type::Bool => "Bool",
        Type::Int(_) => "Int",
        Type::Enum(name) => name,
    }
}

pub(crate) fn term(expr: &Expr) -> String {
    match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::Int(value, _) => number(*value),
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
    }
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

/// One arithmetic operation in code, with the conditions under which it runs.
struct Overflow<'a> {
    expr: &'a Expr,
    site: &'a Site,
    guards: Vec<Guard<'a>>,
}

/// Every arithmetic operation in `expr`, inner ones first. Only the branch
/// of an `if` that runs is guarded by its condition; both operands of `and`
/// and `or` are checked, whatever the other's value.
fn overflows<'a>(expr: &'a Expr, guards: &[Guard<'a>], out: &mut Vec<Overflow<'a>>) {
    match expr {
        Expr::Arith(_, left, right, site) => {
            overflows(left, guards, out);
            overflows(right, guards, out);
            out.push(Overflow {
                expr,
                site,
                guards: guards.to_vec(),
            });
        }
        Expr::Neg(inner, site) => {
            overflows(inner, guards, out);
            out.push(Overflow {
                expr,
                site,
                guards: guards.to_vec(),
            });
        }
        Expr::Not(inner) => overflows(inner, guards, out),
        Expr::Binary(_, left, right) => {
            overflows(left, guards, out);
            overflows(right, guards, out);
        }
        Expr::If(condition, then, otherwise) => {
            overflows(condition, guards, out);
            let guard = |holds| Guard {
                condition: Some(condition),
                holds,
            };
            overflows(then, &[guards, &[guard(true)]].concat(), out);
            overflows(otherwise, &[guards, &[guard(false)]].concat(), out);
        }
        Expr::Bool(_) | Expr::Int(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
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
fn prelude(function: &Function, datatypes: &str) -> String {
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
fn value(function: &Function) -> (String, Option<String>) {
    match &function.body {
        Body::Expr(body) => (term(body), None),
        Body::Decide(rows, _) => (
            decide_term(rows),
            Some(all("or", rows.iter().map(condition).collect(), "false")),
        ),
    }
}

/// The two scripts that judge a mutant of `original` against its contract.
/// `survives` asserts that the mutant breaks some `ensures`, so `unsat` means
/// the contract can't tell it apart. `differs` asserts an admitted input on
/// which the two give different results, `r0` and `r1`, so `unsat` means the
/// mutant is equivalent. Both use `original`'s parameters and `requires`.
pub(crate) fn mutant_scripts(
    original: &Function,
    mutant: &Function,
    datatypes: &str,
) -> (String, String) {
    let prelude = prelude(original, datatypes);
    let params: Vec<String> = original.params.iter().map(|(n, _)| n.clone()).collect();
    let ret = sort(&original.ret);
    let (theirs, their_cover) = value(mutant);
    let (ours, our_cover) = value(original);
    let ensures: Vec<String> = original
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures)
        .map(|c| term(&c.expr))
        .collect();
    let mut breaks = vec![
        format!("(= result {theirs})"),
        format!("(not {})", all("and", ensures, "true")),
    ];
    breaks.extend(their_cover.clone());
    let survives = format!(
        "{prelude}(declare-const result {ret})\n(assert {})\n{}",
        all("and", breaks, "true"),
        ending(&params)
    );
    let mut apart = vec![
        format!("(= r0 {ours})"),
        format!("(= r1 {theirs})"),
        "(not (= r0 r1))".to_string(),
    ];
    apart.extend(our_cover);
    apart.extend(their_cover);
    let mut named = params.clone();
    named.extend(["r0".to_string(), "r1".to_string()]);
    let differs = format!(
        "{prelude}(declare-const r0 {ret})\n(declare-const r1 {ret})\n(assert {})\n{}",
        all("and", apart, "true"),
        ending(&named)
    );
    (survives, differs)
}

/// The script that searches for an open case of `function`: an admitted input
/// with two different results, `r0` and `r1`, that both meet every
/// `ensures`. `sat` finds one; `unsat` means the contract determines the
/// result for every admitted input.
pub(crate) fn open_script(function: &Function, datatypes: &str) -> String {
    let ret = sort(&function.ret);
    let ensures = all(
        "and",
        function
            .clauses
            .iter()
            .filter(|c| c.kind == ClauseKind::Ensures)
            .map(|c| term(&c.expr))
            .collect(),
        "true",
    );
    let mut results = format!("(declare-const r0 {ret})\n(declare-const r1 {ret})\n");
    if let Type::Int(width) = &function.ret {
        for r in ["r0", "r1"] {
            results += &format!("(assert {})\n", in_range(r, *width));
        }
    }
    let mut named: Vec<String> = function.params.iter().map(|(n, _)| n.clone()).collect();
    named.extend(["r0".to_string(), "r1".to_string()]);
    format!(
        "{}(define-fun contract ((result {ret})) Bool {ensures})\n{results}(assert (and (contract r0) (contract r1) (not (= r0 r1))))\n{}",
        prelude(function, datatypes),
        ending(&named)
    )
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

fn obligations_of<'a>(function: &'a Function, datatypes: &str) -> Vec<Obligation<'a>> {
    let prelude = prelude(function, datatypes);
    let params: Vec<String> = function
        .params
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let end = ending(&params);
    let obligation = |clause: &str,
                      line: usize,
                      polarity: Polarity,
                      claim: Claim<'a>,
                      declare: &str,
                      assertion: String| Obligation {
        function,
        clause: clause.to_string(),
        line,
        params: params.clone(),
        polarity,
        claim,
        script: format!("{prelude}{declare}(assert {assertion})\n{end}"),
    };
    let result = format!("(declare-const result {})\n", sort(&function.ret));

    let (body, covered) = value(function);
    let rows = match &function.body {
        Body::Expr(_) => None,
        Body::Decide(rows, line) => Some((rows, *line)),
    };
    let mut out = Vec::new();
    let mut ops = Vec::new();
    if let Body::Expr(body) = &function.body {
        overflows(body, &[], &mut ops);
    }
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures)
    {
        let mut assertion = format!("(and (= result {body}) (not {}))", term(&clause.expr));
        if let Some(covered) = &covered {
            assertion = format!("(and {covered} {assertion})");
        }
        out.push(obligation(
            &clause.text,
            clause.line,
            Polarity::Universal,
            Claim::Ensures(&clause.expr),
            &result,
            assertion,
        ));
    }
    if let (Some((rows, line)), Some(covered)) = (rows, covered) {
        out.push(obligation(
            "every input reaches a row",
            line,
            Polarity::Universal,
            Claim::Covered,
            "",
            format!("(not {covered})"),
        ));
        for (index, row) in rows.iter().enumerate() {
            let mut reaches: Vec<Guard> = rows[..index]
                .iter()
                .map(|earlier| Guard {
                    condition: earlier.condition.as_ref(),
                    holds: false,
                })
                .collect();
            if let Some(c) = &row.condition {
                overflows(c, &reaches, &mut ops);
            }
            reaches.push(Guard {
                condition: row.condition.as_ref(),
                holds: true,
            });
            overflows(&row.result, &reaches, &mut ops);
            out.push(obligation(
                &row.text,
                row.line,
                Polarity::Existential,
                Claim::Reaches(index),
                "",
                all("and", reaches.iter().map(guard_term).collect(), "true"),
            ));
        }
    }
    for op in ops {
        let Some(width) = op.site.width else {
            // `check` sets every width before obligations are generated.
            unreachable!("arithmetic without a width: `{}`", op.site.text)
        };
        let mut assertion: Vec<String> = op.guards.iter().map(guard_term).collect();
        assertion.push(format!("(not {})", in_range(&term(op.expr), width)));
        out.push(obligation(
            &format!("`{}` stays within `{}`", op.site.text, width.name()),
            op.site.line,
            Polarity::Universal,
            Claim::InRange {
                expr: op.expr,
                width,
                guards: op.guards,
            },
            "",
            all("and", assertion, "true"),
        ));
    }
    // Stable, so obligations on one line keep the order above.
    out.sort_by_key(|o| o.line);
    out
}

/// The obligations of every function, in source order: one per `ensures`;
/// for a decision table, that every input reaches a row and that each row
/// can be the first to hold; and that each arithmetic operation in code stays
/// in its type's range.
pub(crate) fn obligations(program: &Program) -> Vec<Obligation<'_>> {
    let datatypes = datatypes(program);
    program
        .functions
        .iter()
        .flat_map(|function| obligations_of(function, &datatypes))
        .collect()
}
