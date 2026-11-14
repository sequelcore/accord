//! The inputs each exported function is called with.

use crate::interpret::{self, Value};
use crate::syntax::{Body, Expr, Function, Machine, Program, Role, Type, Width};
use crate::{Outcome, Report};

pub(super) const LIMIT: usize = 4096;
pub(super) const SAFE: i128 = 9_007_199_254_740_991;

/// A deterministic sequence, so a failure reproduces.
pub(super) struct Seeded(pub(super) u64);

impl Seeded {
    pub(super) fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn int(&mut self, low: i128, high: i128) -> i128 {
        let span = u64::try_from(high - low).unwrap_or(u64::MAX).max(1);
        low + i128::from(self.next() % span)
    }
}

fn texts(expr: &Expr, found: &mut Vec<String>) {
    match expr {
        Expr::Text(text, _) if !found.contains(text) => found.push(text.clone()),
        Expr::List(items, ..) => items.iter().for_each(|i| texts(i, found)),
        Expr::Not(inner) | Expr::Neg(inner, _) => texts(inner, found),
        Expr::Binary(_, l, r) | Expr::Arith(_, l, r, _) => {
            texts(l, found);
            texts(r, found);
        }
        Expr::If(c, a, b) => [c, a, b].iter().for_each(|e| texts(e, found)),
        _ => {}
    }
}

fn literals(program: &Program) -> Vec<String> {
    let mut found = vec![String::new(), "é".to_string(), "\u{1d11e}".to_string()];
    for function in &program.functions {
        function
            .clauses
            .iter()
            .for_each(|c| texts(&c.expr, &mut found));
        match &function.body {
            Body::Expr(e) => texts(e, &mut found),
            Body::Decide(rows, _) => {
                for row in rows {
                    row.condition.iter().for_each(|c| texts(c, &mut found));
                    texts(&row.result, &mut found);
                }
            }
        }
    }
    found
}

fn domain(ty: &Type, program: &Program, words: &[String], seeded: &mut Seeded) -> Vec<Value> {
    let variants = |owner: &str| -> Vec<Value> {
        let e = program
            .enums
            .iter()
            .find(|e| e.name == owner)
            .expect("a checked enum");
        e.variants
            .iter()
            .map(|v| Value::Variant(owner.to_string(), v.clone(), Vec::new()))
            .collect()
    };
    match ty {
        Type::Bool => vec![Value::Bool(true), Value::Bool(false)],
        Type::Int(width) => {
            let (low, high) = match width {
                Width::I32 => (width.min(), width.max()),
                Width::I64 => (-SAFE, SAFE),
            };
            let mut ints = vec![low, low + 1, -1, 0, 1, high - 1, high];
            ints.extend((0..3).map(|_| seeded.int(-1000, 1000)));
            ints.extend((0..2).map(|_| seeded.int(low, high)));
            ints.into_iter().map(Value::Int).collect()
        }
        Type::Text => words.iter().cloned().map(Value::Text).collect(),
        Type::Enum(owner) => variants(owner),
        Type::Record(_) => unreachable!("`build` refuses records"),
        Type::List(element) => {
            let items = domain(element, program, words, seeded);
            let mut lists = vec![Value::List(Vec::new()), Value::List(items.clone())];
            lists.extend(items.into_iter().map(|item| Value::List(vec![item])));
            lists
        }
    }
}

/// Every combination of `domains`, or a seeded sample of `LIMIT` of them.
fn combinations(domains: &[Vec<Value>], seeded: &mut Seeded) -> Vec<Vec<Value>> {
    let total = domains
        .iter()
        .map(Vec::len)
        .try_fold(1usize, usize::checked_mul);
    let pick = |index: usize| -> Vec<Value> {
        let mut rest = index;
        domains
            .iter()
            .map(|d| {
                let v = d[rest % d.len()].clone();
                rest /= d.len();
                v
            })
            .collect()
    };
    match total {
        Some(n) if n <= LIMIT => (0..n).map(pick).collect(),
        _ => (0..LIMIT)
            .map(|_| {
                domains
                    .iter()
                    .map(|d| d[usize::try_from(seeded.next()).unwrap_or(0) % d.len()].clone())
                    .collect()
            })
            .collect(),
    }
}

/// The integer literals in `expr`.
fn integers(expr: &Expr, found: &mut Vec<i128>) {
    match expr {
        Expr::Int(n, _) if !found.contains(n) => found.push(*n),
        Expr::Not(inner) | Expr::Neg(inner, _) => integers(inner, found),
        Expr::Binary(_, l, r) | Expr::Arith(_, l, r, _) => {
            integers(l, found);
            integers(r, found);
        }
        Expr::If(c, a, b) => [c, a, b].iter().for_each(|e| integers(e, found)),
        _ => {}
    }
}

/// Every state, with every set of arguments, that `member` of `machine` is
/// called with: the combinations of its fields' and parameters' values, which
/// include each integer the machine's clauses name and its neighbours, so that
/// some states meet its invariants and others break them. An init takes no
/// state, so its states are empty.
pub(super) fn member_inputs(
    program: &Program,
    machine: &Machine,
    member: &Function,
    seeded: &mut Seeded,
) -> Vec<(Vec<Value>, Vec<Value>)> {
    let words = literals(program);
    let mut named = Vec::new();
    for clause in machine
        .members()
        .flat_map(|m| m.clauses.iter())
        .chain(&machine.invariants)
    {
        integers(&clause.expr, &mut named);
    }
    let fields = member.machine.map_or(0, |m| m.state);
    let reads = member.machine.is_some_and(|m| m.role != Role::Init);
    let types: Vec<&Type> = member.params[if reads { 0 } else { fields }..]
        .iter()
        .map(|(_, ty)| ty)
        .collect();
    let domains: Vec<Vec<Value>> = types
        .iter()
        .map(|ty| {
            let mut values = domain(ty, program, &words, seeded);
            if let Type::Int(width) = ty {
                for n in named.iter().flat_map(|n| [n - 1, *n, n + 1]) {
                    let value = Value::Int(n);
                    if (width.min()..=width.max()).contains(&n) && !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
            values
        })
        .collect();
    let split = if reads { fields } else { 0 };
    combinations(&domains, seeded)
        .into_iter()
        .map(|mut all| {
            let args = all.split_off(split);
            (all, args)
        })
        .collect()
}

/// An `example` argument's value: examples are written with literals.
fn literal(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::Bool(b) => Some(Value::Bool(*b)),
        Expr::Int(n, _) => Some(Value::Int(*n)),
        Expr::Neg(inner, _) => match literal(inner)? {
            Value::Int(n) => Some(Value::Int(-n)),
            _ => None,
        },
        Expr::Text(text, _) => Some(Value::Text(text.clone())),
        Expr::List(items, ..) => items
            .iter()
            .map(literal)
            .collect::<Option<_>>()
            .map(Value::List),
        Expr::Variant(owner, variant, _) => {
            Some(Value::Variant(owner.clone(), variant.clone(), Vec::new()))
        }
        _ => None,
    }
}

/// Every input `function` is called with: the combinations of its
/// parameters' values, each input a solver gave for it, and each of its
/// `example`s' calls.
pub(super) fn inputs(
    program: &Program,
    report: &Report,
    function: &Function,
    seeded: &mut Seeded,
) -> Vec<Vec<Value>> {
    let words = literals(program);
    let domains: Vec<Vec<Value>> = function
        .params
        .iter()
        .map(|(_, ty)| domain(ty, program, &words, seeded))
        .collect();
    let mut all = combinations(&domains, seeded);
    for verdict in report
        .verdicts
        .iter()
        .filter(|v| v.function == function.name)
    {
        if let Outcome::Proved {
            witness: Some(witness),
        } = &verdict.outcome
        {
            let read: Option<Vec<Value>> = function
                .params
                .iter()
                .map(|(n, ty)| {
                    let (_, raw) = witness.iter().find(|(w, _)| w == n)?;
                    interpret::read(raw, ty)
                })
                .collect();
            all.extend(read);
        }
    }
    for example in program
        .examples
        .iter()
        .filter(|e| e.function == function.name)
    {
        let args: Option<Vec<Value>> = function
            .params
            .iter()
            .enumerate()
            .map(|(index, (name, _))| {
                let labelled = example
                    .args
                    .iter()
                    .find(|(label, _)| label.as_ref() == Some(name));
                let (_, expr) = labelled.or_else(|| example.args.get(index))?;
                literal(expr)
            })
            .collect();
        all.push(args.expect("an example's arguments are literals"));
    }
    all
}
