//! A text's length in SMT-LIB: `str.len` is unbounded, but every text holds
//! at most `MAX_TEXT_LENGTH` code points, as a JavaScript string does. That
//! limit is trusted. Each script states it for every length it takes, so
//! arithmetic on a length can be proved to stay within `i64`
//! (docs/design/measurements/2026-11-18-text-length-limit).
//!
//! The limit is stated of each `(str.len X)` an assertion holds, under the
//! quantifiers and `let`s around it, never of a definition's parameter: a
//! limit on every string is one the theory of strings contradicts, and with
//! it every claim would be proved. So each use of a `define-fun` is expanded
//! first, and a length inside a quantifier whose guard isn't the familiar
//! shape gets no limit at all.

use std::collections::BTreeMap;

use crate::syntax::{Body, Expr, Program};

/// The most code points a text holds, which every script assumes.
pub(crate) const MAX_TEXT_LENGTH: u64 = 9_007_199_254_740_991;

/// Whether a unit's proofs can rest on the limit: its function, the first of
/// `program`, takes a text's length, or a callee's contract does.
pub(crate) fn takes_text_length(program: &Program) -> bool {
    fn measures(expr: &Expr) -> bool {
        matches!(expr, Expr::TextLength(..)) || expr.children().into_iter().any(measures)
    }
    let mut functions = program.functions.iter();
    let Some(function) = functions.next() else {
        return false;
    };
    let body: Vec<&Expr> = match &function.body {
        Body::Expr(expr) => vec![expr],
        Body::Decide(rows, _) => rows
            .iter()
            .flat_map(|row| row.condition.iter().chain([&row.result]))
            .collect(),
    };
    function
        .clauses
        .iter()
        .chain(functions.flat_map(|callee| &callee.clauses))
        .map(|clause| &clause.expr)
        .chain(body)
        .any(measures)
}

#[derive(Clone, Debug, PartialEq)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

impl Sexp {
    fn show(&self) -> String {
        match self {
            Sexp::Atom(atom) => atom.clone(),
            Sexp::List(items) => {
                let inner: Vec<String> = items.iter().map(Sexp::show).collect();
                format!("({})", inner.join(" "))
            }
        }
    }

    fn head(&self) -> Option<&str> {
        match self {
            Sexp::List(items) => match items.first() {
                Some(Sexp::Atom(atom)) => Some(atom),
                _ => None,
            },
            Sexp::Atom(_) => None,
        }
    }

    fn items(&self) -> &[Sexp] {
        match self {
            Sexp::List(items) => items,
            Sexp::Atom(_) => &[],
        }
    }
}

/// The script's top-level commands, each with the byte offset it starts at,
/// or `None` if the script doesn't parse.
fn parse(script: &str) -> Option<Vec<(usize, Sexp)>> {
    let bytes = script.as_bytes();
    let mut commands = Vec::new();
    let mut stack: Vec<(usize, Vec<Sexp>)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let atom = match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
                continue;
            }
            b';' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'(' => {
                stack.push((start, Vec::new()));
                i += 1;
                continue;
            }
            b')' => {
                let (opened, items) = stack.pop()?;
                let list = Sexp::List(items);
                match stack.last_mut() {
                    Some((_, outer)) => outer.push(list),
                    None => commands.push((opened, list)),
                }
                i += 1;
                continue;
            }
            b'"' => {
                i += 1;
                loop {
                    match bytes.get(i)? {
                        b'"' if bytes.get(i + 1) == Some(&b'"') => i += 2,
                        b'"' => break,
                        _ => i += 1,
                    }
                }
                i += 1;
                &script[start..i]
            }
            b'|' => {
                i += 1 + script[i + 1..].find('|')? + 1;
                &script[start..i]
            }
            _ => {
                while i < bytes.len() && !b" \t\r\n()".contains(&bytes[i]) {
                    i += 1;
                }
                &script[start..i]
            }
        };
        stack.last_mut()?.1.push(Sexp::Atom(atom.to_string()));
    }
    stack.is_empty().then_some(commands)
}

/// `define-fun`s by name: their parameters and body.
type Definitions = BTreeMap<String, (Vec<String>, Sexp)>;

/// `expr` with every use of a definition replaced by its body, each name the
/// body binds renamed so that no argument is captured.
fn expand(expr: &Sexp, definitions: &Definitions, fresh: &mut usize) -> Sexp {
    match expr {
        Sexp::Atom(name) => match definitions.get(name) {
            Some((params, body)) if params.is_empty() => expand(body, definitions, fresh),
            _ => expr.clone(),
        },
        Sexp::List(items) => {
            let items: Vec<Sexp> = items
                .iter()
                .map(|i| expand(i, definitions, fresh))
                .collect();
            if let Some(Sexp::Atom(name)) = items.first()
                && let Some((params, body)) = definitions.get(name)
                && !params.is_empty()
            {
                let env: BTreeMap<String, Sexp> = params
                    .iter()
                    .cloned()
                    .zip(items[1..].iter().cloned())
                    .collect();
                return expand(&substitute(body, &env, fresh), definitions, fresh);
            }
            Sexp::List(items)
        }
    }
}

/// `expr` with each name in `env` replaced, renaming what `expr` binds.
fn substitute(expr: &Sexp, env: &BTreeMap<String, Sexp>, fresh: &mut usize) -> Sexp {
    let items = match expr {
        Sexp::Atom(name) => return env.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Sexp::List(items) => items,
    };
    let mut rename = |bindings: &Sexp, env: &mut BTreeMap<String, Sexp>| -> Vec<(String, Sexp)> {
        bindings
            .items()
            .iter()
            .filter_map(|binding| {
                let Some(Sexp::Atom(name)) = binding.items().first() else {
                    return None;
                };
                *fresh += 1;
                let renamed = format!("{name}!x{fresh}");
                env.insert(name.clone(), Sexp::Atom(renamed.clone()));
                Some((renamed, binding.items().get(1)?.clone()))
            })
            .collect()
    };
    match (expr.head(), items.get(1), items.get(2)) {
        (Some(q @ ("forall" | "exists")), Some(bindings), Some(body)) => {
            let mut inner = env.clone();
            let renamed = rename(bindings, &mut inner);
            let bindings = renamed
                .into_iter()
                .map(|(name, sort)| Sexp::List(vec![Sexp::Atom(name), sort]))
                .collect();
            Sexp::List(vec![
                Sexp::Atom(q.to_string()),
                Sexp::List(bindings),
                substitute(body, &inner, fresh),
            ])
        }
        (Some("let"), Some(bindings), Some(body)) => {
            let mut inner = env.clone();
            let renamed = rename(bindings, &mut inner);
            let bindings = renamed
                .into_iter()
                .map(|(name, value)| {
                    Sexp::List(vec![Sexp::Atom(name), substitute(&value, env, fresh)])
                })
                .collect();
            Sexp::List(vec![
                Sexp::Atom("let".to_string()),
                Sexp::List(bindings),
                substitute(body, &inner, fresh),
            ])
        }
        _ => Sexp::List(items.iter().map(|i| substitute(i, env, fresh)).collect()),
    }
}

/// What a length is inside: a quantifier, with the guard that keeps its
/// index in range, or a `let`.
#[derive(Clone)]
enum Frame {
    Quantifier { bindings: Sexp, guard: Sexp },
    Let(Sexp),
}

/// Each `(str.len X)` in `expr`, with the frames around it, outermost first.
fn sites(expr: &Sexp, frames: &mut Vec<Frame>, found: &mut Vec<(Sexp, Vec<Frame>)>) {
    let items = expr.items();
    match (expr.head(), items.get(1), items.get(2)) {
        (Some("str.len"), ..) => {
            found.push((expr.clone(), frames.clone()));
            items.iter().skip(1).for_each(|i| sites(i, frames, found));
        }
        (Some("forall" | "exists"), Some(bindings), Some(body)) => {
            // A quantifier over an index guards it as `(=> in-range ...)` or
            // `(and in-range ...)`. Any other shape gets no limit, nor does a
            // quantifier over anything but integers: limiting the lengths of
            // every text it ranges over would limit texts in general.
            let indices = bindings
                .items()
                .iter()
                .all(|b| matches!(b.items(), [_, Sexp::Atom(sort)] if sort == "Int"));
            if let (true, Some("=>" | "and"), [_, guard, inner]) =
                (indices, body.head(), body.items())
            {
                frames.push(Frame::Quantifier {
                    bindings: bindings.clone(),
                    guard: guard.clone(),
                });
                sites(guard, frames, found);
                sites(inner, frames, found);
                frames.pop();
            }
        }
        (Some("let"), Some(bindings), Some(body)) => {
            for binding in bindings.items() {
                binding
                    .items()
                    .iter()
                    .skip(1)
                    .for_each(|v| sites(v, frames, found));
            }
            frames.push(Frame::Let(bindings.clone()));
            sites(body, frames, found);
            frames.pop();
        }
        _ => items.iter().for_each(|i| sites(i, frames, found)),
    }
}

/// That `length` is at most the limit, under `frames`.
fn limit(length: Sexp, frames: &[Frame]) -> Sexp {
    let atom = |s: &str| Sexp::Atom(s.to_string());
    let fact = Sexp::List(vec![atom("<="), length, atom(&MAX_TEXT_LENGTH.to_string())]);
    frames.iter().rev().fold(fact, |fact, frame| match frame {
        Frame::Let(bindings) => Sexp::List(vec![atom("let"), bindings.clone(), fact]),
        Frame::Quantifier { bindings, guard } => Sexp::List(vec![
            atom("forall"),
            bindings.clone(),
            Sexp::List(vec![atom("=>"), guard.clone(), fact]),
        ]),
    })
}

/// `script` with the length limit asserted, before its first `check-sat`,
/// for every text length its assertions take. A script that doesn't parse,
/// or takes no length, is returned as it is.
pub(crate) fn limit_lengths(script: &str) -> String {
    if !script.contains("str.len") {
        return script.to_string();
    }
    let Some(commands) = parse(script) else {
        return script.to_string();
    };
    let mut definitions = Definitions::new();
    let mut facts: Vec<String> = Vec::new();
    let mut fresh = 0;
    let mut check = None;
    for (start, command) in &commands {
        let items = command.items();
        match (command.head(), items) {
            (Some("define-fun"), [_, Sexp::Atom(name), params, _, body]) => {
                let names = params
                    .items()
                    .iter()
                    .filter_map(|p| match p.items().first() {
                        Some(Sexp::Atom(n)) => Some(n.clone()),
                        _ => None,
                    })
                    .collect();
                definitions.insert(name.clone(), (names, body.clone()));
            }
            (Some("assert"), [_, claim]) => {
                let mut found = Vec::new();
                sites(
                    &expand(claim, &definitions, &mut fresh),
                    &mut Vec::new(),
                    &mut found,
                );
                for (length, frames) in found {
                    let fact = format!("(assert {})\n", limit(length, &frames).show());
                    if !facts.contains(&fact) {
                        facts.push(fact);
                    }
                }
            }
            (Some("check-sat"), _) => {
                check = Some(*start);
                break;
            }
            _ => {}
        }
    }
    match check {
        Some(at) if !facts.is_empty() => {
            format!("{}{}{}", &script[..at], facts.concat(), &script[at..])
        }
        _ => script.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::limit_lengths;

    const LIMIT: &str = "9007199254740991";

    #[test]
    fn limits_a_ground_length_before_check_sat() {
        let script =
            "(declare-const w String)\n(assert (not (<= (+ (str.len w) 1) 10)))\n(check-sat)\n";
        let limited = limit_lengths(script);
        let fact = format!("(assert (<= (str.len w) {LIMIT}))\n(check-sat)");
        assert!(limited.contains(&fact), "{limited}");
    }

    #[test]
    fn limits_a_length_inside_a_quantifier_under_its_guard() {
        let script = "(declare-const xs (Seq String))\n(assert (forall ((k Int)) (=> (and (<= 0 k) (< k (seq.len xs))) (let ((w (seq.nth xs k))) (> (str.len w) 0)))))\n(check-sat)\n";
        let limited = limit_lengths(script);
        let fact = format!(
            "(assert (forall ((k Int)) (=> (and (<= 0 k) (< k (seq.len xs))) (let ((w (seq.nth xs k))) (<= (str.len w) {LIMIT})))))"
        );
        assert!(limited.contains(&fact), "{limited}");
    }

    #[test]
    fn never_limits_a_definition_parameter_only_its_uses() {
        let script = "(declare-const a String)\n(define-fun e ((t String) (r Int)) Bool (= r (str.len t)))\n(declare-const r Int)\n(assert (e a r))\n(check-sat)\n";
        let limited = limit_lengths(script);
        assert!(
            limited.contains(&format!("(assert (<= (str.len a) {LIMIT}))")),
            "{limited}"
        );
        assert!(!limited.contains("(str.len t)) 9"), "{limited}");
        assert!(!limited.contains("(forall ((t String))"), "{limited}");
    }

    #[test]
    fn expanding_a_definition_never_captures_an_argument() {
        // `e`'s own `w` must not capture the caller's `w` passed as `t`.
        let script = "(declare-const xs (Seq String))\n(declare-const w String)\n(define-fun e ((t String)) Bool (forall ((k Int)) (=> (< k 1) (let ((w (seq.nth xs k))) (= (str.len t) (str.len w))))))\n(assert (e w))\n(check-sat)\n";
        let limited = limit_lengths(script);
        let facts: Vec<&str> = limited
            .lines()
            .filter(|l| l.starts_with("(assert (forall"))
            .collect();
        assert!(
            facts
                .iter()
                .any(|f| f.contains(&format!("(<= (str.len w) {LIMIT})"))),
            "{limited}"
        );
        assert!(facts.iter().all(|f| !f.contains("(let ((w (")), "{limited}");
    }

    #[test]
    fn leaves_a_quantifier_of_another_shape_unlimited() {
        let script = "(declare-const xs (Seq String))\n(assert (forall ((k Int)) (> (str.len (seq.nth xs k)) 0)))\n(check-sat)\n";
        assert_eq!(limit_lengths(script), script);
    }

    #[test]
    fn leaves_a_quantifier_over_texts_unlimited() {
        let script = "(assert (forall ((s String)) (=> (> (str.len s) 3) (> (str.len s) 2))))\n(check-sat)\n";
        assert_eq!(limit_lengths(script), script);
    }

    #[test]
    fn reads_a_text_literal_as_one_atom() {
        let script = "(declare-const w String)\n(assert (= w \"(check-sat) \"\")\"))\n(assert (> (str.len w) 0))\n(check-sat)\n";
        let limited = limit_lengths(script);
        let fact = format!("(assert (<= (str.len w) {LIMIT}))\n(check-sat)\n");
        assert!(limited.ends_with(&fact), "{limited}");
    }
}
