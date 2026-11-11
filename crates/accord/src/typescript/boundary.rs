//! The exported function: it checks what the proofs assume of its caller,
//! its parameters' types and ranges and each `requires`, and then calls the
//! body. Integers in a `requires` are mathematical, so they are `bigint`s.

use super::expr::{Context, Scope, camel, string, value};
use super::{enumeration, outer, signature};
use crate::syntax::{ClauseKind, Function, Program, Type, Width};

pub(super) const TEXT_CHECK: &str = "function isText$(value: string): boolean {
  for (const character of value) {
    const point = character.codePointAt(0)!;
    if ((point >= 0xd800 && point <= 0xdfff) || point > 0x2ffff) {
      return false;
    }
  }
  return true;
}
";

const TEXT_RANGE: &str = "must hold only characters up to U+2FFFF, with no lone surrogates";

fn throw(check: &str, error: &str, message: &str) -> String {
    format!(
        "  if ({check}) {{\n    throw new {error}({});\n  }}\n",
        string(message)
    )
}

/// The exported function, and whether it checks a text.
pub(super) fn wrapper(program: &Program, function: &Function) -> (String, bool) {
    let name = camel(&function.name);
    let mut out = format!("export function {} {{\n", signature(function, &name, outer));
    let mut text = false;
    for (param, ty) in &function.params {
        let p = camel(param);
        let must = |what: &str| format!("{name}: {p} must be {what}");
        let variants = |enum_name: &str| -> String {
            let e = enumeration(program, enum_name).expect("`check` resolves every enum");
            let values: Vec<String> = e.variants.iter().map(|v| value(v)).collect();
            format!("[{}]", values.join(", "))
        };
        match ty {
            Type::Bool => {
                out += &throw(
                    &format!("typeof {p} !== \"boolean\""),
                    "TypeError",
                    &must("a Bool"),
                );
            }
            Type::Int(width) => {
                let label = format!("an {}", width.name());
                out += &throw(
                    &format!("typeof {p} !== \"number\""),
                    "TypeError",
                    &must(&label),
                );
                out += &match width {
                    Width::I32 => throw(
                        &format!(
                            "!Number.isInteger({p}) || {p} < {} || {p} > {}",
                            width.min(),
                            width.max()
                        ),
                        "RangeError",
                        &must(&label),
                    ),
                    Width::I64 => throw(
                        &format!("!Number.isSafeInteger({p})"),
                        "RangeError",
                        &must("a safe integer"),
                    ),
                };
            }
            Type::Text => {
                text = true;
                out += &throw(
                    &format!("typeof {p} !== \"string\""),
                    "TypeError",
                    &must("a Text"),
                );
                out += &throw(
                    &format!("!isText$({p})"),
                    "RangeError",
                    &format!("{name}: {p} {TEXT_RANGE}"),
                );
            }
            Type::Enum(e) => {
                out += &throw(
                    &format!("!{}.includes({p})", variants(e)),
                    "TypeError",
                    &must(&format!("a {e}")),
                );
            }
            Type::List(element) => {
                let item = match element.as_ref() {
                    Type::Bool => "typeof item === \"boolean\"".to_string(),
                    Type::Text => "typeof item === \"string\"".to_string(),
                    Type::Enum(e) => format!("{}.includes(item)", variants(e)),
                    other => unreachable!("`check` admits no List<{}>", other.name()),
                };
                let check = format!("!Array.isArray({p}) || !{p}.every((item) => {item})");
                out += &throw(&check, "TypeError", &must(&format!("a {}", ty.name())));
                if **element == Type::Text {
                    text = true;
                    out += &throw(
                        &format!("!{p}.every(isText$)"),
                        "RangeError",
                        &format!("{name}: {p} {TEXT_RANGE}"),
                    );
                }
            }
        }
    }
    let spec = Scope {
        params: &function.params,
        context: Context::Spec,
    };
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        let check = format!("!({})", spec.bare(&clause.expr, None));
        out += &throw(&check, "RangeError", &format!("{name}: {}", clause.text));
    }
    let args: Vec<String> = function
        .params
        .iter()
        .map(|(n, t)| match t {
            Type::Int(Width::I64) => format!("BigInt({})", camel(n)),
            _ => camel(n),
        })
        .collect();
    let call = format!("{name}$({})", args.join(", "));
    if function.ret == Type::Int(Width::I64) {
        out += &format!("  const result = {call};\n");
        out += &throw(
            "result < -9007199254740991n || result > 9007199254740991n",
            "RangeError",
            &format!("{name}: the result is not a safe integer"),
        );
        out += "  return Number(result);\n";
    } else {
        out += &format!("  return {call};\n");
    }
    out += "}\n";
    (out, text)
}
