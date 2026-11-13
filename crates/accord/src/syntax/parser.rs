//! The parser: tokens to a [`Program`].

use super::token::{Token, tokenize};
use super::{Program, Site};
use crate::Diagnostic;

pub(super) struct Parser<'a> {
    pub(super) source: Vec<&'a str>,
    pub(super) tokens: Vec<(Token, usize, usize)>,
    pub(super) pos: usize,
}

impl Parser<'_> {
    pub(super) fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .or(self.tokens.last())
            .map_or(1, |t| t.1)
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.0)
    }

    pub(super) fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w == word)
    }

    pub(super) fn at_sym(&self, sym: &str) -> bool {
        matches!(self.peek(), Some(Token::Sym(s)) if *s == sym)
    }

    pub(super) fn error<T>(&self, message: impl Into<String>) -> Result<T, Diagnostic> {
        Err(Diagnostic::new(self.line(), message))
    }

    pub(super) fn expect_sym(&mut self, sym: &str) -> Result<(), Diagnostic> {
        if self.at_sym(sym) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected `{sym}`"))
        }
    }

    pub(super) fn expect_word(&mut self, word: &str) -> Result<(), Diagnostic> {
        if self.at_word(word) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected `{word}`"))
        }
    }

    pub(super) fn name(&mut self) -> Result<String, Diagnostic> {
        match self.peek() {
            Some(Token::Word(w)) if !is_keyword(w) => {
                let w = w.clone();
                self.pos += 1;
                Ok(w)
            }
            _ => self.error("expected a name"),
        }
    }

    /// The source text from the current token to the end of its line,
    /// without a trailing comment, with the token's line.
    pub(super) fn rest_of_line(&self) -> (String, usize) {
        let (line, column) = (self.tokens[self.pos].1, self.tokens[self.pos].2);
        let text = self.source[line - 1][column..]
            .split("//")
            .next()
            .unwrap_or_default()
            .trim();
        (text.to_string(), line)
    }

    /// The source text of tokens `start..self.pos`, as written when they are
    /// on one line.
    fn text_since(&self, start: usize) -> String {
        let tokens = &self.tokens[start..self.pos];
        let len = |token: &Token| match token {
            Token::Word(w) => w.len(),
            Token::Sym(s) => s.len(),
            Token::Str(s) => s.len() + 2,
        };
        match (tokens.first(), tokens.last()) {
            (Some(first), Some(last)) if first.1 == last.1 => {
                self.source[first.1 - 1][first.2..last.2 + len(&last.0)].to_string()
            }
            _ => tokens
                .iter()
                .map(|t| match &t.0 {
                    Token::Word(w) => w.clone(),
                    Token::Sym(s) => (*s).to_string(),
                    Token::Str(s) => format!("\"{s}\""),
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    pub(super) fn site(&self, start: usize) -> Site {
        Site {
            width: None,
            text: self.text_since(start),
            line: self.tokens[start].1,
        }
    }
}

pub(super) fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "fn" | "enum"
            | "example"
            | "pub"
            | "intent"
            | "open"
            | "when"
            | "in"
            | "requires"
            | "ensures"
            | "decide"
            | "otherwise"
            | "if"
            | "else"
            | "true"
            | "false"
            | "and"
            | "or"
            | "not"
            | "implies"
            | "result"
            | "all"
            | "any"
            | "length"
            | "indices"
    )
}

/// Parses every enum and function in `source`.
pub(crate) fn parse(source: &str) -> Result<Program, Diagnostic> {
    let mut parser = Parser {
        source: source.lines().collect(),
        tokens: tokenize(source)?,
        pos: 0,
    };
    let mut program = Program {
        enums: Vec::new(),
        functions: Vec::new(),
        machines: Vec::new(),
        examples: Vec::new(),
        machine_examples: Vec::new(),
    };
    while parser.peek().is_some() {
        // Within a folder every file sees every declaration; `pub` decides
        // what a file's generated module exports.
        let public = parser.at_word("pub");
        if public {
            parser.pos += 1;
        }
        if parser.at_word("enum") {
            program.enums.push(parser.enumeration()?);
        } else if parser.at_word("machine") {
            let mut machine = parser.machine()?;
            machine.public = public;
            program.machines.push(machine);
        } else if parser.at_word("example") {
            // `example f(…) == v` calls a function; `example M` runs a machine.
            if matches!(
                parser.tokens.get(parser.pos + 2),
                Some((Token::Sym("("), ..))
            ) {
                program.examples.push(parser.example()?);
            } else {
                program.machine_examples.push(parser.machine_example()?);
            }
        } else {
            let mut function = parser.function()?;
            function.public = public;
            program.functions.push(function);
        }
    }
    Ok(program)
}
