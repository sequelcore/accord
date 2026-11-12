//! Evaluating expressions and bodies, in code or with mathematical integers.

use super::{Value, broken_requires};
use crate::obligation::{Guard, term};
use crate::syntax::{Arith, Body, Expr, Function, Op};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Code,
    Math,
}

pub(super) struct Env<'a> {
    pub(super) names: &'a [(String, Value)],
    pub(super) result: Option<&'a Value>,
    pub(super) mode: Mode,
    /// The functions a call may run.
    pub(super) functions: &'a [Function],
    /// When replaying a model, each call's value as the model gives it, by
    /// the call's term; a call then takes that value instead of running.
    pub(super) assumed: Option<&'a [(String, Value)]>,
}

impl<'a> Env<'a> {
    /// An environment with no functions to call and no assumed values.
    pub(super) fn plain(
        names: &'a [(String, Value)],
        result: Option<&'a Value>,
        mode: Mode,
    ) -> Env<'a> {
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
    pub(super) fn eval(&self, expr: &Expr) -> Result<Value, String> {
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

    pub(super) fn boolean(&self, expr: &Expr) -> Result<bool, String> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            _ => Err("expected a `Bool`".to_string()),
        }
    }

    pub(super) fn int(&self, expr: &Expr) -> Result<i128, String> {
        match self.eval(expr)? {
            Value::Int(n) => Ok(n),
            _ => Err("expected an integer".to_string()),
        }
    }

    pub(super) fn guard(&self, guard: &Guard) -> Result<bool, String> {
        let value = match guard.condition {
            Some(condition) => self.boolean(condition)?,
            None => true,
        };
        Ok(value == guard.holds)
    }
}

/// The index of the first row whose condition holds, if any.
pub(super) fn first_row(function: &Function, env: &Env) -> Result<Option<usize>, String> {
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

pub(super) fn body(function: &Function, env: &Env) -> Result<Value, String> {
    match &function.body {
        Body::Expr(body) => env.eval(body),
        Body::Decide(rows, _) => match first_row(function, env)? {
            Some(index) => env.eval(&rows[index].result),
            None => Err("no row matches".to_string()),
        },
    }
}
