//! Splits source text into words and symbols, each with its line and column.

use crate::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    Word(String),
    Sym(&'static str),
}

/// Longer symbols first, so `->` and `=>` aren't read as two symbols.
const SYMBOLS: [&str; 12] = [
    "->", "=>", "==", "!=", "(", ")", "{", "}", ":", ",", ".", "|",
];

pub(super) fn tokenize(source: &str) -> Result<Vec<(Token, usize, usize)>, Diagnostic> {
    let mut tokens = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = raw.split("//").next().unwrap_or_default();
        let mut rest = text;
        while let Some(start) = rest.find(|c: char| !c.is_whitespace()) {
            let at = text.len() - rest.len() + start;
            rest = &rest[start..];
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
                return Err(Diagnostic::new(
                    line,
                    format!("unexpected character `{found}`"),
                ));
            }
            tokens.push((Token::Word(rest[..len].to_string()), line, at));
            rest = &rest[len..];
        }
    }
    Ok(tokens)
}
