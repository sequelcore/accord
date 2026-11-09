//! The parser: tokens to a [`Program`].

use super::token::{Token, tokenize};
use super::{Body, Clause, ClauseKind, Enum, Example, Function, Program, Row, Site, Type, Width};
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
    fn rest_of_line(&self) -> (String, usize) {
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

    fn ty(&mut self) -> Result<Type, Diagnostic> {
        let name = self.name()?;
        Ok(match name.as_str() {
            "Bool" => Type::Bool,
            "i32" => Type::Int(Width::I32),
            "i64" => Type::Int(Width::I64),
            "Text" => Type::Text,
            _ => Type::Enum(name),
        })
    }

    fn enumeration(&mut self) -> Result<Enum, Diagnostic> {
        let line = self.line();
        self.expect_word("enum")?;
        let name = self.name()?;
        self.expect_sym("{")?;
        let mut variants = Vec::new();
        while !self.at_sym("}") {
            variants.push(self.name()?);
            if !self.at_sym("}") {
                self.expect_sym(",")?;
            }
        }
        self.expect_sym("}")?;
        if variants.is_empty() {
            return Err(Diagnostic::new(line, format!("`{name}` has no variants")));
        }
        Ok(Enum {
            name,
            variants,
            line,
        })
    }

    fn function(&mut self) -> Result<Function, Diagnostic> {
        let line = self.line();
        self.expect_word("fn")?;
        let name = self.name()?;
        self.expect_sym("(")?;
        let mut params = Vec::new();
        while !self.at_sym(")") {
            let param = self.name()?;
            self.expect_sym(":")?;
            params.push((param, self.ty()?));
            if !self.at_sym(")") {
                self.expect_sym(",")?;
            }
        }
        self.expect_sym(")")?;
        self.expect_sym("->")?;
        let ret = self.ty()?;
        if self.at_word("intent") {
            // An intent is for people: nothing checks it, so nothing keeps it.
            self.pos += 1;
            match self.peek() {
                Some(Token::Str(_)) => self.pos += 1,
                _ => return self.error("expected the intent as a string"),
            }
        }
        let mut clauses = Vec::new();
        while self.at_word("requires") || self.at_word("ensures") || self.at_word("open") {
            let kind = if self.at_word("requires") {
                ClauseKind::Requires
            } else if self.at_word("ensures") {
                ClauseKind::Ensures
            } else {
                ClauseKind::Open
            };
            let (text, line) = self.rest_of_line();
            self.pos += 1;
            if kind == ClauseKind::Open {
                self.expect_word("when")?;
            }
            let expr = self.expr()?;
            clauses.push(Clause {
                kind,
                expr,
                text,
                line,
            });
        }
        let body = if self.at_word("decide") {
            self.decide()?
        } else {
            self.expect_sym("{")?;
            let body = self.expr()?;
            self.expect_sym("}")?;
            Body::Expr(body)
        };
        Ok(Function {
            name,
            line,
            params,
            ret,
            clauses,
            body,
        })
    }

    fn example(&mut self) -> Result<Example, Diagnostic> {
        let (text, line) = self.rest_of_line();
        self.expect_word("example")?;
        let function = self.name()?;
        self.expect_sym("(")?;
        let mut args = Vec::new();
        while !self.at_sym(")") {
            let labelled = matches!(self.tokens.get(self.pos + 1), Some((Token::Sym(":"), ..)));
            let label = if labelled {
                let label = self.name()?;
                self.expect_sym(":")?;
                Some(label)
            } else {
                None
            };
            args.push((label, self.expr()?));
            if !self.at_sym(")") {
                self.expect_sym(",")?;
            }
        }
        self.expect_sym(")")?;
        self.expect_sym("==")?;
        let expected = self.expr()?;
        // A call written over several lines reads as one.
        let last = self.tokens[self.pos - 1].1;
        let text = if last == line {
            text
        } else {
            self.source[line - 1..last]
                .iter()
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join(" ")
        };
        Ok(Example {
            function,
            args,
            expected,
            text,
            line,
        })
    }

    fn decide(&mut self) -> Result<Body, Diagnostic> {
        let line = self.line();
        self.expect_word("decide")?;
        self.expect_sym("{")?;
        let mut rows: Vec<Row> = Vec::new();
        while self.at_sym("|") {
            if rows.last().is_some_and(|row| row.condition.is_none()) {
                return self.error("`otherwise` must be the last row");
            }
            let (text, row_line) = self.rest_of_line();
            self.pos += 1;
            let condition = if self.at_word("otherwise") {
                self.pos += 1;
                None
            } else {
                Some(self.expr()?)
            };
            self.expect_sym("=>")?;
            rows.push(Row {
                condition,
                result: self.expr()?,
                text,
                line: row_line,
            });
        }
        if rows.is_empty() {
            return self.error("expected a row starting with `|`");
        }
        self.expect_sym("}")?;
        Ok(Body::Decide(rows, line))
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
        examples: Vec::new(),
    };
    while parser.peek().is_some() {
        // `pub` has no effect until one file can import another.
        if parser.at_word("pub") {
            parser.pos += 1;
        }
        if parser.at_word("enum") {
            program.enums.push(parser.enumeration()?);
        } else if parser.at_word("example") {
            program.examples.push(parser.example()?);
        } else {
            program.functions.push(parser.function()?);
        }
    }
    Ok(program)
}
