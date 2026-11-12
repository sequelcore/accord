//! Replaying a solver's model, so no counterexample or witness is taken on
//! the solver's word alone.

use super::eval::{Env, Mode, body, first_row};
use super::{Names, Value, admitted, broken_requires, read};
use crate::Inputs;
use crate::obligation::{Claim, Guard, Obligation, term};
use crate::syntax::{Expr, Range};

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
        } => runs(&env, guards)? && !(width.min()..=width.max()).contains(&env.int(expr)?),
        Claim::Requires { call, guards } => {
            let runs = runs(&env, guards)?;
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
        Claim::InBounds {
            index,
            guards,
            ensures,
        } => {
            let result = if *ensures {
                Some(body(function, &env)?)
            } else {
                None
            };
            let env = Env {
                result: result.as_ref(),
                ..env
            };
            let Expr::Index(list, at, _) = index else {
                return Ok(false);
            };
            runs(&env, guards)? && {
                let length = env.list(list)?.len() as i128;
                !(0..length).contains(&env.int(at)?)
            }
        }
    })
}

/// Whether every guard holds, each evaluated whatever the others give.
fn runs(env: &Env, guards: &[Guard]) -> Result<bool, String> {
    let mut runs = true;
    for guard in guards {
        runs &= env.guard(guard)?;
    }
    Ok(runs)
}

/// `names` with each quantifier's name bound to the element or index the
/// model gives, outermost first, provided each index is in range.
fn bind(obligation: &Obligation, inputs: &Inputs, mut names: Names) -> Result<Names, String> {
    for (index, quantified) in &obligation.bound {
        let Expr::Quantified(_, name, range, ..) = quantified else {
            continue;
        };
        let raw = inputs
            .iter()
            .find(|(n, _)| n == index)
            .map(|(_, v)| v.as_str())
            .ok_or(format!("no value for `{index}`"))?;
        let at: usize = raw.parse().map_err(|_| format!("`{raw}` isn't an index"))?;
        let env = Env {
            names: &names,
            result: None,
            mode: Mode::Math,
            functions: obligation.functions,
            assumed: None,
        };
        let items = env.list(range.list())?;
        let item = items
            .get(at)
            .cloned()
            .ok_or(format!("the model's index {at} is out of range"))?;
        let value = match range {
            Range::Elements(_) => item,
            Range::Indices(_) => Value::Int(at as i128),
        };
        names.push((name.clone(), value));
    }
    Ok(names)
}

/// Replays a solver's model of an obligation's script: how the inputs do
/// what the solver says they do (break a universal claim, or satisfy an
/// existential one), or why they don't.
pub(crate) fn replay(obligation: &Obligation, inputs: &Inputs) -> Result<Replay, String> {
    let names = bind(obligation, inputs, admitted(obligation.function, inputs)?)?;
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
