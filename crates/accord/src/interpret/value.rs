//! Values, and reading a solver's value for a parameter.

use crate::syntax::Type;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Value {
    Bool(bool),
    Int(i128),
    Text(String),
    List(Vec<Value>),
    /// An enum's variant: the enum, the variant, and its fields, each with
    /// its name, in their declared order.
    Variant(String, String, Vec<(String, Value)>),
    /// A record: its name, and its fields, each with its name, in their
    /// declared order.
    Record(String, Vec<(String, Value)>),
}

/// Fields as Accord writes them: `name: value`, separated by commas.
fn labelled(fields: &[(String, Value)]) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{name}: {}", value.source()))
        .collect::<Vec<_>>()
        .join(", ")
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
            Value::Variant(owner, name, fields) if fields.is_empty() => format!("{owner}.{name}"),
            Value::Variant(owner, name, fields) => format!("{owner}.{name}({})", labelled(fields)),
            Value::Record(name, fields) => format!("{name} {{ {} }}", labelled(fields)),
        }
    }
}

/// Reads one value as [`Value::source`] writes it, from `text` at `at`.
/// A text ends at a quote that the end or `,`, `)`, `]` or `}` follows, so
/// a quote inside one stays part of it.
struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn rest(&self) -> &str {
        &self.text[self.at..]
    }

    fn skip_space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    fn eat(&mut self, symbol: &str) -> bool {
        self.skip_space();
        if self.rest().starts_with(symbol) {
            self.at += symbol.len();
            true
        } else {
            false
        }
    }

    fn word(&mut self) -> Option<String> {
        self.skip_space();
        let rest = self.rest();
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(rest.len());
        let word = rest[..len].to_string();
        self.at += len;
        (len > 0).then_some(word)
    }

    /// `name: value, ...` up to `close`.
    fn fields(&mut self, close: &str) -> Option<Vec<(String, Value)>> {
        let mut fields = Vec::new();
        while !self.eat(close) {
            if !fields.is_empty() && !self.eat(",") {
                return None;
            }
            let name = self.word()?;
            if !self.eat(":") {
                return None;
            }
            fields.push((name, self.value()?));
        }
        Some(fields)
    }

    fn value(&mut self) -> Option<Value> {
        self.skip_space();
        if self.eat("\"") {
            let rest = self.rest();
            let end = rest.char_indices().find(|(i, c)| {
                *c == '"'
                    && rest[i + 1..]
                        .trim_start()
                        .chars()
                        .next()
                        .is_none_or(|next| matches!(next, ',' | ')' | ']' | '}'))
            })?;
            let text = rest[..end.0].to_string();
            self.at += end.0 + 1;
            return Some(Value::Text(text));
        }
        if self.eat("[") {
            let mut items = Vec::new();
            while !self.eat("]") {
                if !items.is_empty() && !self.eat(",") {
                    return None;
                }
                items.push(self.value()?);
            }
            return Some(Value::List(items));
        }
        let word = self.word()?;
        match word.as_str() {
            "true" => return Some(Value::Bool(true)),
            "false" => return Some(Value::Bool(false)),
            _ => {}
        }
        if let Ok(n) = word.parse() {
            return Some(Value::Int(n));
        }
        if self.eat(".") {
            let variant = self.word()?;
            let fields = if self.eat("(") {
                self.fields(")")?
            } else {
                Vec::new()
            };
            return Some(Value::Variant(word, variant, fields));
        }
        if self.eat("{") {
            return Some(Value::Record(word, self.fields("}")?));
        }
        None
    }
}

/// Whether `value` has the shape of `ty`: its own kind, a list's elements
/// each, and an enum's or record's name.
fn shaped(value: &Value, ty: &Type) -> bool {
    match (value, ty) {
        (Value::Bool(_), Type::Bool)
        | (Value::Int(_), Type::Int(_))
        | (Value::Text(_), Type::Text) => true,
        (Value::List(items), Type::List(element)) => items.iter().all(|i| shaped(i, element)),
        (Value::Variant(owner, ..), Type::Enum(name))
        | (Value::Record(owner, _), Type::Record(name)) => owner == name,
        _ => false,
    }
}

/// Reads a solver's value for a parameter of type `ty`, written as
/// [`Value::source`] writes it.
pub(crate) fn read(value: &str, ty: &Type) -> Option<Value> {
    // A text is everything between its quotes, whatever it holds.
    if *ty == Type::Text {
        return value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .map(|v| Value::Text(v.to_string()));
    }
    let mut reader = Reader { text: value, at: 0 };
    let read = reader.value()?;
    reader.skip_space();
    (reader.rest().is_empty() && shaped(&read, ty)).then_some(read)
}
