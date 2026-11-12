//! The interpreter: it runs `example`s, and replays each model a solver gives
//! so no counterexample or witness is taken on the solver's word alone.
//!
//! It evaluates in one of two modes. Code that runs is evaluated with each
//! arithmetic result checked against its width; specifications, and the
//! solvers' scripts, use mathematical integers.
//!
//! A call runs the callee's body, after checking its `requires`. A solver's
//! model of a caller is replayed twice when it doesn't reproduce as run:
//! once more with each call taking the value the model gives it, which tells
//! a counterexample to the code from one that rests on a callee's contract
//! being weaker than its body.

use crate::Inputs;
use crate::obligation::{Claim, Guard, Obligation, term};
use crate::syntax::{Arith, Body, ClauseKind, Example, Expr, Function, Op, Type};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Bool(bool),
    Int(i128),
    Text(String),
    List(Vec<Value>),
    /// An enum's variant: the enum, then the variant.
    Variant(String, String),
}

impl Value {
    /// The value as Accord source.
    pub fn source(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Text(t) => format!("\"{t}\""),
            Value::List(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(Value::source)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Variant(owner, name) => format!("{owner}.{name}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Code,
    Math,
}

struct Env<'a> {
    names: &'a [(String, Value)],
    result: Option<&'a Value>,
    mode: Mode,
    /// The functions a call may run.
    functions: &'a [Function],
    /// When replaying a model, each call's value as the model gives it, by
    /// the call's term; a call then takes that value instead of running.
    assumed: Option<&'a [(String, Value)]>,
}

impl<'a> Env<'a> {
    /// An environment with no functions to call and no assumed values.
    fn plain(names: &'a [(String, Value)], result: Option<&'a Value>, mode: Mode) -> Env<'a> {
        Env {
            names,
            result,
            mode,
            functions: &[],
            assumed: None,
        }
    }
}

impl Env<'_> {
    fn eval(&self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Int(n, _) => Ok(Value::Int(*n)),
            Expr::Text(t, _) => Ok(Value::Text(t.clone())),
            Expr::List(items, ..) => Ok(Value::List(
                items
                    .iter()
                    .map(|i| self.eval(i))
                    .collect::<Result<_, _>>()?,
            )),
            Expr::Variant(owner, name, _) => Ok(Value::Variant(owner.clone(), name.clone())),
            Expr::Name(name, _) if name == "result" => self
                .result
                .cloned()
                .ok_or("`result` has no value here".to_string()),
            Expr::Name(name, _) => self
                .names
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .ok_or(format!("`{name}` has no value")),
            Expr::Not(inner) => Ok(Value::Bool(!self.boolean(inner)?)),
            Expr::Binary(op, left, right) => {
                // Both operands are evaluated, as overflow is checked in both.
                let (l, r) = (self.eval(left)?, self.eval(right)?);
                let ints = || match (&l, &r) {
                    (Value::Int(a), Value::Int(b)) => Ok((*a, *b)),
                    _ => Err("an ordering needs integers".to_string()),
                };
                let bools = || match (&l, &r) {
                    (Value::Bool(a), Value::Bool(b)) => Ok((*a, *b)),
                    _ => Err("a connective needs `Bool` values".to_string()),
                };
                Ok(Value::Bool(match op {
                    Op::In => match &r {
                        Value::List(items) => items.contains(&l),
                        _ => return Err("`in` needs a list".to_string()),
                    },
                    Op::Eq => l == r,
                    Op::Ne => l != r,
                    Op::Lt => ints().map(|(a, b)| a < b)?,
                    Op::Le => ints().map(|(a, b)| a <= b)?,
                    Op::Gt => ints().map(|(a, b)| a > b)?,
                    Op::Ge => ints().map(|(a, b)| a >= b)?,
                    Op::And => bools().map(|(a, b)| a && b)?,
                    Op::Or => bools().map(|(a, b)| a || b)?,
                    Op::Implies => bools().map(|(a, b)| !a || b)?,
                }))
            }
            Expr::If(condition, then, otherwise) => {
                if self.boolean(condition)? {
                    self.eval(then)
                } else {
                    self.eval(otherwise)
                }
            }
            Expr::Arith(op, left, right, site) => {
                let (a, b) = (self.int(left)?, self.int(right)?);
                let value = match op {
                    Arith::Add => a.checked_add(b),
                    Arith::Sub => a.checked_sub(b),
                    Arith::Mul => a.checked_mul(b),
                }
                .ok_or(format!("`{}` is too large to evaluate", site.text))?;
                self.fits(value, site)
            }
            Expr::Neg(inner, site) => {
                let value = self.int(inner)?.checked_neg();
                self.fits(
                    value.ok_or(format!("`{}` is too large to evaluate", site.text))?,
                    site,
                )
            }
            Expr::Call(name, args, site) => {
                if let Some(assumed) = self.assumed {
                    let key = term(expr);
                    return assumed
                        .iter()
                        .find(|(t, _)| *t == key)
                        .map(|(_, v)| v.clone())
                        .ok_or(format!("the model gives `{}` no value", site.text));
                }
                let Some(callee) = self.functions.iter().find(|f| f.name == *name) else {
                    return Err(format!("`{name}` can't be run here"));
                };
                let names = callee
                    .params
                    .iter()
                    .zip(args)
                    .map(|((param, _), (_, arg))| Ok((param.clone(), self.eval(arg)?)))
                    .collect::<Result<Vec<_>, String>>()?;
                if let Some(clause) = broken_requires(callee, &names)? {
                    return Err(format!("`{}` breaks `{clause}`", site.text));
                }
                let env = Env {
                    names: &names,
                    result: None,
                    mode: self.mode,
                    functions: self.functions,
                    assumed: None,
                };
                body(callee, &env)
            }
        }
    }

    fn fits(&self, value: i128, site: &crate::syntax::Site) -> Result<Value, String> {
        match site.width {
            Some(width)
                if self.mode == Mode::Code && !(width.min()..=width.max()).contains(&value) =>
            {
                Err(format!("`{}` overflows `{}`", site.text, width.name()))
            }
            _ => Ok(Value::Int(value)),
        }
    }

    fn boolean(&self, expr: &Expr) -> Result<bool, String> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            _ => Err("expected a `Bool`".to_string()),
        }
    }

    fn int(&self, expr: &Expr) -> Result<i128, String> {
        match self.eval(expr)? {
            Value::Int(n) => Ok(n),
            _ => Err("expected an integer".to_string()),
        }
    }

    fn guard(&self, guard: &Guard) -> Result<bool, String> {
        let value = match guard.condition {
            Some(condition) => self.boolean(condition)?,
            None => true,
        };
        Ok(value == guard.holds)
    }
}

/// The index of the first row whose condition holds, if any.
fn first_row(function: &Function, env: &Env) -> Result<Option<usize>, String> {
    let Body::Decide(rows, _) = &function.body else {
        return Ok(None);
    };
    for (index, row) in rows.iter().enumerate() {
        let holds = match &row.condition {
            Some(condition) => env.boolean(condition)?,
            None => true,
        };
        if holds {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn body(function: &Function, env: &Env) -> Result<Value, String> {
    match &function.body {
        Body::Expr(body) => env.eval(body),
        Body::Decide(rows, _) => match first_row(function, env)? {
            Some(index) => env.eval(&rows[index].result),
            None => Err("no row matches".to_string()),
        },
    }
}

/// The first `requires` that doesn't hold, as written.
pub(crate) fn broken_requires(
    function: &Function,
    names: &[(String, Value)],
) -> Result<Option<String>, String> {
    let env = Env::plain(names, None, Mode::Math);
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        if !env.boolean(&clause.expr)? {
            return Ok(Some(clause.text.clone()));
        }
    }
    Ok(None)
}

/// Runs an example: `Ok` when the call gives the expected result, otherwise
/// why it doesn't. `functions` are those its function may call.
pub(crate) fn run(
    example: &Example,
    function: &Function,
    functions: &[Function],
) -> Result<(), String> {
    let constants = Env::plain(&[], None, Mode::Code);
    let names = example
        .args
        .iter()
        .map(|(label, expr)| Ok((label.clone().unwrap_or_default(), constants.eval(expr)?)))
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(clause) = broken_requires(function, &names)? {
        return Err(format!("the call breaks `{clause}`"));
    }
    let env = Env {
        names: &names,
        result: None,
        mode: Mode::Code,
        functions,
        assumed: None,
    };
    let got = body(function, &env)?;
    let want = constants.eval(&example.expected)?;
    if got == want {
        Ok(())
    } else {
        Err(format!("it gives {}, not {}", got.source(), want.source()))
    }
}

/// A list literal's elements as written, split at its top-level commas; a
/// comma inside a text is part of the text.
fn split_list(value: &str) -> Option<Vec<String>> {
    let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?;
    let (mut items, mut current, mut quoted) = (Vec::new(), String::new(), false);
    for c in inner.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            ',' if !quoted => items.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() || !items.is_empty() {
        items.push(current);
    }
    Some(items.into_iter().map(|i| i.trim().to_string()).collect())
}

/// Reads a solver's value for a parameter of type `ty`.
pub(crate) fn read(value: &str, ty: &Type) -> Option<Value> {
    match ty {
        Type::Bool => value.parse().ok().map(Value::Bool),
        Type::Int(_) => value.parse().ok().map(Value::Int),
        Type::Text => value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .map(|v| Value::Text(v.to_string())),
        Type::List(element) => split_list(value)?
            .iter()
            .map(|item| read(item, element))
            .collect::<Option<Vec<_>>>()
            .map(Value::List),
        Type::Enum(owner) => value
            .strip_prefix(&format!("{owner}."))
            .map(|name| Value::Variant(owner.clone(), name.to_string())),
    }
}

/// Replays a solver's model of an obligation's script: `Ok` when the inputs
/// do what the solver says they do (break a universal claim, or satisfy an
/// existential one), otherwise why they don't.
/// A solver's values for `function`'s parameters, read as values, provided
/// every `requires` holds of them.
pub(crate) fn admitted(function: &Function, inputs: &Inputs) -> Result<Names, String> {
    let names = function
        .params
        .iter()
        .map(|(name, ty)| {
            let raw = inputs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
                .ok_or(format!("no value for `{name}`"))?;
            read(raw, ty)
                .map(|value| (name.clone(), value))
                .ok_or(format!("`{raw}` isn't a value of `{name}`'s type"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(clause) = broken_requires(function, &names)? {
        return Err(format!("it breaks `{clause}`"));
    }
    Ok(names)
}

/// Parameter names with their values.
pub(crate) type Names = Vec<(String, Value)>;

/// `function`'s result for `names`, with mathematical integers; it may call
/// `functions`.
pub(crate) fn result(
    function: &Function,
    names: &Names,
    functions: &[Function],
) -> Result<Value, String> {
    let env = Env {
        names,
        result: None,
        mode: Mode::Math,
        functions,
        assumed: None,
    };
    body(function, &env)
}

/// Whether some `open when` of `function` covers `names`.
pub(crate) fn covered(function: &Function, names: &Names) -> Result<bool, String> {
    let env = Env::plain(names, None, Mode::Math);
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Open)
    {
        if env.boolean(&clause.expr)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether every `ensures` of `contract` holds of `result` for `names`.
pub(crate) fn meets(contract: &Function, names: &Names, result: &Value) -> Result<bool, String> {
    let env = Env::plain(names, Some(result), Mode::Math);
    for clause in contract
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Ensures)
    {
        if !env.boolean(&clause.expr)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// How a solver's model of an obligation reproduces when the interpreter
/// replays it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Replay {
    /// It does what the solver says when the code runs as written.
    Runs,
    /// It does only when each call takes the value the model gives it: the
    /// model rests on a callee's contract allowing a result its body never
    /// gives.
    Assumed,
}

/// Whether `inputs` do what the solver says of `obligation`, with each call
/// running, or taking the model's value when `assumed` is given.
fn reproduces(
    obligation: &Obligation,
    names: &Names,
    assumed: Option<&[(String, Value)]>,
) -> Result<bool, String> {
    let function = obligation.function;
    let env = Env {
        names,
        result: None,
        mode: Mode::Math,
        functions: obligation.functions,
        assumed,
    };
    Ok(match &obligation.claim {
        Claim::Ensures(clause) => {
            let result = body(function, &env)?;
            let with_result = Env {
                result: Some(&result),
                ..env
            };
            !with_result.boolean(clause)?
        }
        Claim::Covered => first_row(function, &env)?.is_none(),
        Claim::Reaches(index) => first_row(function, &env)? == Some(*index),
        Claim::InRange {
            expr,
            width,
            guards,
        } => {
            let mut runs = true;
            for guard in guards {
                runs &= env.guard(guard)?;
            }
            runs && !(width.min()..=width.max()).contains(&env.int(expr)?)
        }
        Claim::Requires { call, guards } => {
            let mut runs = true;
            for guard in guards {
                runs &= env.guard(guard)?;
            }
            let Expr::Call(name, args, _) = call else {
                return Ok(false);
            };
            let Some(callee) = obligation.functions.iter().find(|f| f.name == *name) else {
                return Ok(false);
            };
            let names = callee
                .params
                .iter()
                .zip(args)
                .map(|((param, _), (_, arg))| Ok((param.clone(), env.eval(arg)?)))
                .collect::<Result<Vec<_>, String>>()?;
            runs && broken_requires(callee, &names)?.is_some()
        }
    })
}

/// Replays a solver's model of an obligation's script: how the inputs do
/// what the solver says they do (break a universal claim, or satisfy an
/// existential one), or why they don't.
pub(crate) fn replay(obligation: &Obligation, inputs: &Inputs) -> Result<Replay, String> {
    let names = admitted(obligation.function, inputs)?;
    if reproduces(obligation, &names, None)? {
        return Ok(Replay::Runs);
    }
    if obligation.calls.is_empty() {
        return Err("the interpreter disagrees".to_string());
    }
    let mut assumed = Vec::new();
    for (constant, call) in &obligation.calls {
        let Expr::Call(name, ..) = call else {
            continue;
        };
        let Some(callee) = obligation.functions.iter().find(|f| f.name == *name) else {
            continue;
        };
        let raw = inputs
            .iter()
            .find(|(n, _)| n == constant)
            .map(|(_, v)| v.as_str())
            .ok_or(format!("no value for `{constant}`"))?;
        let value = read(raw, &callee.ret).ok_or(format!("`{raw}` isn't a result of `{name}`"))?;
        assumed.push((term(call), value));
    }
    if reproduces(obligation, &names, Some(&assumed))? {
        Ok(Replay::Assumed)
    } else {
        Err("the interpreter disagrees".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::replay;
    use crate::obligation::{Obligation, obligations};
    use crate::{check, syntax};

    fn inputs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect()
    }

    /// Replays `model` against the obligation whose clause contains `clause`.
    fn replays(source: &str, clause: &str, model: &[(&str, &str)]) -> Result<(), String> {
        let mut program = syntax::parse(source).expect("parses");
        check::check(&mut program, &[]).expect("checks");
        let all: Vec<Obligation> = obligations(&program);
        let obligation = all
            .iter()
            .find(|o| o.clause.contains(clause))
            .expect("the obligation exists");
        replay(obligation, &inputs(model)).map(|_| ())
    }

    const TABLE: &str = "enum Light { Red, Amber, Green }
fn next(light: Light) -> Light
    ensures result != light
decide {
    | light == Red   => Green
    | light == Green => Amber
}
";

    #[test]
    fn a_counterexample_to_an_ensures_must_break_it() {
        let source = "fn add(a: i32, b: i32) -> i32
    ensures result > a
{
    a + b
}
";
        assert!(replays(source, "ensures", &[("a", "1"), ("b", "0")]).is_ok());
        assert!(replays(source, "ensures", &[("a", "1"), ("b", "5")]).is_err());
    }

    #[test]
    fn a_counterexample_to_coverage_must_reach_no_row() {
        assert!(replays(TABLE, "every input", &[("light", "Light.Amber")]).is_ok());
        assert!(replays(TABLE, "every input", &[("light", "Light.Red")]).is_err());
    }

    #[test]
    fn a_witness_must_reach_its_row_first() {
        assert!(replays(TABLE, "light == Green", &[("light", "Light.Green")]).is_ok());
        assert!(replays(TABLE, "light == Green", &[("light", "Light.Red")]).is_err());
    }

    #[test]
    fn an_overflow_counterexample_must_overflow_where_the_code_runs() {
        let source = "fn step(a: i32) -> i32
{
    if a < 0 { a } else { a + 1 }
}
";
        assert!(replays(source, "stays within", &[("a", "2147483647")]).is_ok());
        assert!(replays(source, "stays within", &[("a", "5")]).is_err());
        // Overflows, but on the branch that doesn't run.
        let guarded = "fn step(a: i32) -> i32
{
    if a > 0 { a } else { a - 1 }
}
";
        assert!(replays(guarded, "stays within", &[("a", "2147483647")]).is_err());
    }

    #[test]
    fn a_model_must_admit_every_requires_and_read_as_a_value() {
        let source = "fn add(a: i32, b: i32) -> i32
    requires a > 0
    ensures result > a
{
    a + b
}
";
        assert!(replays(source, "ensures", &[("a", "-1"), ("b", "0")]).is_err());
        assert!(replays(source, "ensures", &[("a", "x"), ("b", "0")]).is_err());
        assert!(replays(source, "ensures", &[("a", "1")]).is_err());
    }
}
