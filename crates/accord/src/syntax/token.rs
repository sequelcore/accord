//! Splits source text into words, symbols and strings, each with its line
//! and column. `//` starts a comment, except inside a string.

use crate::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    Word(String),
    Sym(&'static str),
    /// A string literal's contents, without its quotes. It has no escapes.
    Str(String),
}

/// Longer symbols first, so `->`, `<=` and the like aren't read as two.
const SYMBOLS: [&str; 22] = [
    "->", "=>", "==", "!=", "<=", ">=", "<", ">", "=", "+", "-", "*", "(", ")", "{", "}", "[", "]",
    ":", ",", ".", "|",
];

pub(super) fn tokenize(source: &str) -> Result<Vec<(Token, usize, usize)>, Diagnostic> {
    let mut tokens = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = raw;
        let mut rest = text;
        while let Some(start) = rest.find(|c: char| !c.is_whitespace()) {
            let at = text.len() - rest.len() + start;
            rest = &rest[start..];
            if rest.starts_with("//") {
                break;
            }
            if let Some(inner) = rest.strip_prefix('"') {
                let Some(end) = inner.find('"') else {
                    return Err(Diagnostic::new(line, "a string must end on its line"));
                };
                // SMT-LIB's strings hold characters up to U+2FFFF only.
                if let Some(c) = inner[..end].chars().find(|c| u32::from(*c) > 0x2FFFF) {
                    return Err(Diagnostic::new(
                        line,
                        format!(
                            "a text can't hold U+{:X}: the solvers reason about characters up to U+2FFFF",
                            u32::from(c)
                        ),
                    ));
                }
                tokens.push((Token::Str(inner[..end].to_string()), line, at));
                rest = &inner[end + 1..];
                continue;
            }
            if let Some(sym) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) {
                tokens.push((Token::Sym(sym), line, at));
                rest = &rest[sym.len()..];
                continue;
            }
            let len = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            if len == 0 {
                let found = rest.chars().next().unwrap_or(' ');
                let message = if matches!(found, '/' | '%') {
                    format!("`{found}` isn't supported yet")
                } else {
                    format!("unexpected character `{found}`")
                };
                return Err(Diagnostic::new(line, message));
            }
            tokens.push((Token::Word(rest[..len].to_string()), line, at));
            rest = &rest[len..];
        }
    }
    Ok(tokens)
}
