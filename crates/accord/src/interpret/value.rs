//! Values, and reading a solver's value for a parameter.

use crate::syntax::Type;

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
