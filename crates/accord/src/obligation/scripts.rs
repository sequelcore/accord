//! Scripts for analyses beside obligations: judging a mutant, and searching
//! for an open case.

use super::calls::{call_facts, declarations};
use super::lists::list_facts;
use super::operations::calls_in;
use super::smt::{all, ending, in_range, prelude, sort, term, value};
use crate::syntax::{ClauseKind, Function, Program, Type};

/// The two scripts that judge a mutant of `original` against its contract.
/// `survives` asserts that the mutant breaks some `ensures`, so `unsat` means
/// the contract can't tell it apart. `differs` asserts an admitted input on
/// which the two give different results, `r0` and `r1`, so `unsat` means the
/// mutant is equivalent. Both use `original`'s parameters and `requires`,
/// and know the functions either body calls, in `program`, only through
/// their contracts.
pub(crate) fn mutant_scripts(
    original: &Function,
    mutant: &Function,
    program: &Program,
    datatypes: &str,
) -> (String, String) {
    let mut calls = Vec::new();
    for function in [original, mutant] {
        calls_in(function, &mut calls);
    }
    let mut callees = Vec::new();
    for call in &calls {
        crate::check::callees(call.expr, &mut callees);
    }
    let prelude = format!(
        "{}{}{}{}",
        prelude(original, datatypes),
        list_facts(&[original, mutant]),
        declarations(program, &callees),
        call_facts(program, &calls).0
    );
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
/// no `open when` covers, with two different results, `r0` and `r1`, that
/// both meet every `ensures`. `sat` finds one; `unsat` means the contract determines the
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
    let free = all(
        "or",
        function
            .clauses
            .iter()
            .filter(|c| c.kind == ClauseKind::Open)
            .map(|c| term(&c.expr))
            .collect(),
        "false",
    );
    let mut named: Vec<String> = function.params.iter().map(|(n, _)| n.clone()).collect();
    named.extend(["r0".to_string(), "r1".to_string()]);
    format!(
        "{}{}(define-fun contract ((result {ret})) Bool {ensures})\n{results}(assert (and (not {free}) (contract r0) (contract r1) (not (= r0 r1))))\n{}",
        prelude(function, datatypes),
        list_facts(&[function]),
        ending(&named)
    )
}
