//! The exported function: it checks what the proofs assume of its caller,
//! its parameters' types and ranges and each `requires`, and then calls the
//! body. Integers in a `requires` are mathematical, so they are `bigint`s.
//!
//! A machine's state is a `readonly` object of its fields. An event or
//! observer takes it first and checks it, its fields and then every
//! invariant and helper invariant, before it checks anything else.

use super::expr::{Context, Scope, camel, string};
use super::{enumeration, outer};
use crate::syntax::{ClauseKind, Function, Machine, Program, Role, Type, Width};

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

/// A template literal that starts with `${where}: `, so a check of a state
/// says which exported function it runs for. The message is escaped.
fn template(message: &str) -> String {
    let mut out = String::from("`${where$}: ");
    for c in message.chars() {
        if matches!(c, '\\' | '`' | '$') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('`');
    out
}

/// An `if` that throws `error` with the message, a literal, when `check`.
fn throw(check: &str, error: &str, message: &str) -> String {
    format!("  if ({check}) {{\n    throw new {error}({message});\n  }}\n")
}

/// How a check words what it found.
struct Words<'a> {
    /// What the value is called in a message, given its variable.
    subject: &'a dyn Fn(&str) -> String,
    /// A message as a TypeScript literal.
    render: &'a dyn Fn(&str) -> String,
}

/// The checks of `params`' types and ranges, one `if` each, and whether any
/// is of a text. Each parameter is a variable of its camel-case name.
fn param_checks(program: &Program, params: &[(String, Type)], words: &Words) -> (String, bool) {
    let mut out = String::new();
    let mut text = false;
    for (param, ty) in params {
        let p = camel(param);
        let subject = (words.subject)(&p);
        let must = |what: &str| (words.render)(&format!("{subject} must be {what}"));
        let variants = |enum_name: &str| -> String {
            let e = enumeration(program, enum_name).expect("`check` resolves every enum");
            let values: Vec<String> = e.variants.iter().map(|v| string(&e.text(v))).collect();
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
                    &(words.render)(&format!("{subject} {TEXT_RANGE}")),
                );
            }
            Type::Enum(e) => {
                out += &throw(
                    &format!("!{}.includes({p})", variants(e)),
                    "TypeError",
                    &must(&format!("a {e}")),
                );
            }
            Type::Record(_) => unreachable!("`build` refuses records before writing"),
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
                        &(words.render)(&format!("{subject} {TEXT_RANGE}")),
                    );
                }
            }
        }
    }
    (out, text)
}

/// The fields as a comma-separated list of camel-case names.
fn field_names(machine: &Machine) -> String {
    let names: Vec<String> = machine.state.iter().map(|(n, _)| camel(n)).collect();
    names.join(", ")
}

/// The machine's state as a type: its fields, in order.
pub(super) fn state_type(machine: &Machine) -> String {
    let mut out = format!("export type {} = {{\n", machine.name);
    for (field, ty) in &machine.state {
        out += &format!("  readonly {}: {};\n", camel(field), outer(ty));
    }
    out + "};\n"
}

/// The function that checks a state: that it is an object with the fields'
/// types and ranges, and that it meets every invariant and then every helper
/// invariant. Also whether it checks a text.
pub(super) fn state_check(program: &Program, machine: &Machine) -> (String, bool) {
    let name = &machine.name;
    let mut out = format!("function check{name}$(where$: string, state$: {name}): {name} {{\n");
    out += &throw(
        "typeof state$ !== \"object\" || state$ === null",
        "TypeError",
        &template(&format!("state must be a {name}")),
    );
    out += &format!("  const {{ {} }} = state$;\n", field_names(machine));
    let (checks, text) = param_checks(
        program,
        &machine.state,
        &Words {
            subject: &|field| format!("state.{field}"),
            render: &template,
        },
    );
    out += &checks;
    let spec = Scope {
        params: &machine.state,
        context: Context::Spec,
        functions: &[],
        enums: &program.enums,
        indices: &[],
    };
    for helper in [false, true] {
        for invariant in machine.invariants.iter().filter(|c| c.helper == helper) {
            let check = format!("!({})", spec.bare(&invariant.expr, None));
            out += &throw(
                &check,
                "RangeError",
                &template(&format!("state breaks {}", invariant.text)),
            );
        }
    }
    out += &format!("  return {{ {} }};\n", field_names(machine));
    (out + "}\n", text)
}

/// The exported function of a function, or of a machine's init, event or
/// observer, and whether it checks a text. An event or observer takes the
/// machine's state first; an init has none to take.
pub(super) fn wrapper(
    program: &Program,
    function: &Function,
    machine: Option<&Machine>,
) -> (String, bool) {
    let name = camel(&function.name);
    let role = function.machine.map(|m| m.role);
    let state = function.machine.map_or(0, |m| m.state);
    let reads_state = role.is_some_and(|r| r != Role::Init);
    let own = &function.params[state..];
    let ret = match (role, machine) {
        (Some(Role::Init | Role::Event), Some(machine)) => machine.name.clone(),
        _ => outer(&function.ret),
    };
    let mut params: Vec<String> = Vec::new();
    if let (true, Some(machine)) = (reads_state, machine) {
        params.push(format!("state$: {}", machine.name));
    }
    params.extend(
        own.iter()
            .map(|(n, t)| format!("{}: {}", camel(n), outer(t))),
    );
    let mut out = format!("export function {name}({}): {ret} {{\n", params.join(", "));
    if let (true, Some(machine)) = (reads_state, machine) {
        out += &format!(
            "  const {{ {} }} = check{}$({}, state$);\n",
            field_names(machine),
            machine.name,
            string(&name)
        );
    }
    let (checks, text) = param_checks(
        program,
        own,
        &Words {
            subject: &|p| p.to_string(),
            render: &|message| string(&format!("{name}: {message}")),
        },
    );
    out += &checks;
    let spec = Scope {
        params: &function.params,
        context: Context::Spec,
        functions: &[],
        enums: &program.enums,
        indices: &[],
    };
    for clause in function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
    {
        let check = format!("!({})", spec.bare(&clause.expr, None));
        out += &throw(
            &check,
            "RangeError",
            &string(&format!("{name}: {}", clause.text)),
        );
    }
    let args: Vec<String> = function.params[if role == Some(Role::Init) { state } else { 0 }..]
        .iter()
        .map(|(n, t)| match t {
            Type::Int(Width::I64) => format!("BigInt({})", camel(n)),
            _ => camel(n),
        })
        .collect();
    let call = format!("{name}$({})", args.join(", "));
    let unsafe_fields: Vec<String> = match (role, machine) {
        (Some(Role::Init | Role::Event), Some(machine)) => machine
            .state
            .iter()
            .filter(|(_, ty)| *ty == Type::Int(Width::I64))
            .map(|(field, _)| format!("!Number.isSafeInteger(result.{})", camel(field)))
            .collect(),
        _ => Vec::new(),
    };
    if function.machine.is_none_or(|m| m.role == Role::Observer)
        && function.ret == Type::Int(Width::I64)
    {
        out += &format!("  const result = {call};\n");
        out += &throw(
            "result < -9007199254740991n || result > 9007199254740991n",
            "RangeError",
            &string(&format!("{name}: the result is not a safe integer")),
        );
        out += "  return Number(result);\n";
    } else if unsafe_fields.is_empty() {
        out += &format!("  return {call};\n");
    } else {
        out += &format!("  const result = {call};\n");
        out += &throw(
            &unsafe_fields.join(" || "),
            "RangeError",
            &string(&format!(
                "{name}: the state has a field that is not a safe integer"
            )),
        );
        out += "  return result;\n";
    }
    out += "}\n";
    (out, text)
}
