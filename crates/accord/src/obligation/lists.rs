//! Lists in SMT-LIB: a list is a sequence. Quantifiers, `length` and
//! indexing read its elements by index. `in` is `seq.contains`, which the
//! solvers decide well alone, or, over a literal list, a disjunction; a
//! script that also reads lists by index states how each `in` relates to
//! the indices, since the solvers don't connect the two on their own.
//!
//! `count` has no definition a solver can use without induction, so each
//! one is a constant that only its axioms constrain: it lies between zero
//! and the list's length, it is zero exactly when no element meets the
//! condition, and it is the length exactly when every one does. The axioms
//! are trusted. A list holds at most `MAX_LENGTH` elements, which is also
//! trusted: it is the most a TypeScript array can hold.

use super::operations::for_every;
use super::smt::{all, term};
use crate::identity::hash;
use crate::syntax::{Body, Expr, Function, Op, Quantifier, Range};

/// The most elements a list holds, which every script assumes.
pub(crate) const MAX_LENGTH: u64 = 4_294_967_295;

/// That `index` is an index of `list`.
pub(crate) fn in_bounds(list: &str, index: &str) -> String {
    format!("(and (<= 0 {index}) (< {index} (seq.len {list})))")
}

/// The name of the index a quantifier ranges over: its own name for
/// `indices`, and a name of its own for elements, which nothing in Accord
/// can spell.
pub(crate) fn index_name(name: &str, range: &Range) -> String {
    match range {
        Range::Indices(_) => name.to_string(),
        Range::Elements(_) => format!("{name}!"),
    }
}

/// `condition` where the quantifier's name stands for the element at its
/// index; for `indices`, the name is the index itself.
pub(crate) fn at_index(name: &str, range: &Range, condition: &str) -> String {
    match range {
        Range::Indices(_) => condition.to_string(),
        Range::Elements(list) => format!(
            "(let (({name} (seq.nth {} {name}!))) {condition})",
            term(list)
        ),
    }
}

/// `x in list`: one of a literal list's elements, or `seq.contains`.
pub(crate) fn contains(list: &Expr, element: &str) -> String {
    if let Expr::List(items, ..) = list {
        let each = items
            .iter()
            .map(|item| format!("(= {element} {})", term(item)))
            .collect();
        return all("or", each, "false");
    }
    format!("(seq.contains {} (seq.unit {element}))", term(list))
}

/// A quantifier as a term. `count` is its constant.
pub(crate) fn quantified(
    quantifier: Quantifier,
    name: &str,
    range: &Range,
    condition: &Expr,
) -> String {
    if quantifier == Quantifier::Count {
        return count_name(name, range, condition);
    }
    let list = term(range.list());
    let index = index_name(name, range);
    let inside = in_bounds(&list, &index);
    let holds = at_index(name, range, &term(condition));
    match quantifier {
        Quantifier::Any => format!("(exists (({index} Int)) (and {inside} {holds}))"),
        _ => format!("(forall (({index} Int)) (=> {inside} {holds}))"),
    }
}

/// The constant a `count` stands for, named by what it counts.
fn count_name(name: &str, range: &Range, condition: &Expr) -> String {
    let counted = format!("{name} {} {}", term(range.list()), term(condition));
    format!("count!{}", &hash(&counted)[..16])
}

/// Visits every expression in `expr`, with the quantifiers it is inside.
fn walk<'a>(expr: &'a Expr, bound: &[&'a Expr], visit: &mut impl FnMut(&'a Expr, &[&'a Expr])) {
    visit(expr, bound);
    let mut each = |inner: &'a Expr| walk(inner, bound, visit);
    match expr {
        Expr::Quantified(_, _, range, condition, _) => {
            each(range.list());
            walk(condition, &[bound, &[expr]].concat(), visit);
        }
        Expr::Not(inner) | Expr::Neg(inner, _) | Expr::Length(inner, _) => each(inner),
        Expr::Binary(_, left, right)
        | Expr::Arith(_, left, right, _)
        | Expr::Index(left, right, _) => {
            each(left);
            each(right);
        }
        Expr::If(condition, then, otherwise) => {
            each(condition);
            each(then);
            each(otherwise);
        }
        Expr::List(items, ..) => items.iter().for_each(each),
        Expr::Call(_, args, _) => args.iter().for_each(|(_, arg)| each(arg)),
        Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {}
    }
}

/// Visits every expression in `functions`' clauses and bodies.
fn each_expr<'a>(functions: &[&'a Function], visit: &mut impl FnMut(&'a Expr, &[&'a Expr])) {
    for function in functions {
        for clause in &function.clauses {
            walk(&clause.expr, &[], visit);
        }
        match &function.body {
            Body::Expr(body) => walk(body, &[], visit),
            Body::Decide(rows, _) => {
                for row in rows {
                    if let Some(condition) = &row.condition {
                        walk(condition, &[], visit);
                    }
                    walk(&row.result, &[], visit);
                }
            }
        }
    }
}

/// The facts a script needs about lists in `functions`: each `count`'s
/// declaration and axioms and, where a list is read by index, how each `in`
/// over a list that isn't literal relates to its indices. Each is stated
/// once.
pub(crate) fn list_facts(functions: &[&Function]) -> String {
    let (mut counts, mut members, mut by_index) = (Vec::new(), Vec::new(), false);
    each_expr(functions, &mut |expr, bound| match expr {
        Expr::Quantified(Quantifier::Count, ..) => counts.push(expr),
        Expr::Quantified(..) | Expr::Index(..) => by_index = true,
        Expr::Binary(Op::In, _, list) if !matches!(**list, Expr::List(..)) => {
            members.push((expr, bound.to_vec()));
        }
        _ => {}
    });
    by_index |= !counts.is_empty();
    let (mut out, mut seen) = (String::new(), Vec::new());
    for expr in counts {
        let Expr::Quantified(_, name, range, condition, _) = expr else {
            continue;
        };
        let constant = count_name(name, range, condition);
        if seen.contains(&constant) {
            continue;
        }
        let list = term(range.list());
        let some = quantified(Quantifier::Any, name, range, condition);
        let every = quantified(Quantifier::All, name, range, condition);
        out += &format!(
            "(declare-const {constant} Int)\n(assert (and (<= 0 {constant}) (<= {constant} (seq.len {list}))))\n(assert (= (= {constant} 0) (not {some})))\n(assert (= (= {constant} (seq.len {list})) {every}))\n"
        );
        seen.push(constant);
    }
    if !by_index {
        return out;
    }
    for (expr, bound) in members {
        let Expr::Binary(_, element, list) = expr else {
            continue;
        };
        let (element, list) = (term(element), term(list));
        let fact = format!(
            "(= (seq.contains {list} (seq.unit {element})) (exists ((in! Int)) (and {} (= (seq.nth {list} in!) {element}))))",
            in_bounds(&list, "in!")
        );
        let fact = for_every(&bound, fact);
        if !seen.contains(&fact) {
            out += &format!("(assert {fact})\n");
            seen.push(fact);
        }
    }
    out
}
