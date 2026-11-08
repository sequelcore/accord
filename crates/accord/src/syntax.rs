//! Tokens and the parser for the subset Accord compiles today: functions over
//! `Bool` with `requires`, `ensures` and an expression body.

use crate::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Type {
    Bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    And,
    Or,
    Implies,
    Eq,
    Ne,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    Bool(bool),
    Name(String, usize),
    Not(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClauseKind {
    Requires,
    Ensures,
}

#[derive(Clone, Debug)]
pub(crate) struct Clause {
    pub kind: ClauseKind,
    pub expr: Expr,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub clauses: Vec<Clause>,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    Sym(&'static str),
}

const SYMBOLS: [&str; 9] = ["->", "==", "!=", "(", ")", "{", "}", ":", ","];

fn tokenize(source: &str) -> Result<Vec<(Token, usize, usize)>, Diagnostic> {
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

struct Parser<'a> {
    source: Vec<&'a str>,
    tokens: Vec<(Token, usize, usize)>,
    pos: usize,
}

impl Parser<'_> {
    fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .or(self.tokens.last())
            .map_or(1, |t| t.1)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.0)
    }

    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w == word)
    }

    fn at_sym(&self, sym: &str) -> bool {
        matches!(self.peek(), Some(Token::Sym(s)) if *s == sym)
    }

    fn error<T>(&self, message: impl Into<String>) -> Result<T, Diagnostic> {
        Err(Diagnostic::new(self.line(), message))
    }

    fn expect_sym(&mut self, sym: &str) -> Result<(), Diagnostic> {
        if self.at_sym(sym) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected `{sym}`"))
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), Diagnostic> {
        if self.at_word(word) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected `{word}`"))
        }
    }

    fn name(&mut self) -> Result<String, Diagnostic> {
        match self.peek() {
            Some(Token::Word(w)) if !is_keyword(w) => {
                let w = w.clone();
                self.pos += 1;
                Ok(w)
            }
            _ => self.error("expected a name"),
        }
    }

    fn ty(&mut self) -> Result<Type, Diagnostic> {
        let name = self.name()?;
        match name.as_str() {
            "Bool" => Ok(Type::Bool),
            other => self.error(format!("type `{other}` isn't supported yet")),
        }
    }

    fn function(&mut self) -> Result<Function, Diagnostic> {
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
        let mut clauses = Vec::new();
        while self.at_word("requires") || self.at_word("ensures") {
            let kind = if self.at_word("requires") {
                ClauseKind::Requires
            } else {
                ClauseKind::Ensures
            };
            let (line, column) = (self.tokens[self.pos].1, self.tokens[self.pos].2);
            self.pos += 1;
            let expr = self.expr()?;
            let text = self.source[line - 1][column..]
                .split("//")
                .next()
                .unwrap_or_default()
                .trim();
            clauses.push(Clause {
                kind,
                expr,
                text: text.to_string(),
                line,
            });
        }
        self.expect_sym("{")?;
        let body = self.expr()?;
        self.expect_sym("}")?;
        Ok(Function {
            name,
            params,
            ret,
            clauses,
            body,
        })
    }

    fn expr(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.or()?;
        if self.at_word("implies") {
            self.pos += 1;
            let right = self.expr()?;
            return Ok(Expr::Binary(Op::Implies, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.and()?;
        while self.at_word("or") {
            self.pos += 1;
            left = Expr::Binary(Op::Or, Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.not()?;
        while self.at_word("and") {
            self.pos += 1;
            left = Expr::Binary(Op::And, Box::new(left), Box::new(self.not()?));
        }
        Ok(left)
    }

    fn not(&mut self) -> Result<Expr, Diagnostic> {
        if self.at_word("not") {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.primary()?;
        let op = if self.at_sym("==") {
            Op::Eq
        } else if self.at_sym("!=") {
            Op::Ne
        } else {
            return Ok(left);
        };
        self.pos += 1;
        Ok(Expr::Binary(op, Box::new(left), Box::new(self.primary()?)))
    }

    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        if self.at_sym("(") {
            self.pos += 1;
            let inner = self.expr()?;
            self.expect_sym(")")?;
            return Ok(inner);
        }
        if self.at_word("if") {
            self.pos += 1;
            let condition = self.expr()?;
            self.expect_sym("{")?;
            let then = self.expr()?;
            self.expect_sym("}")?;
            self.expect_word("else")?;
            self.expect_sym("{")?;
            let otherwise = self.expr()?;
            self.expect_sym("}")?;
            return Ok(Expr::If(
                Box::new(condition),
                Box::new(then),
                Box::new(otherwise),
            ));
        }
        if self.at_word("true") || self.at_word("false") {
            let value = self.at_word("true");
            self.pos += 1;
            return Ok(Expr::Bool(value));
        }
        if self.at_word("result") {
            self.pos += 1;
            return Ok(Expr::Name(
                "result".to_string(),
                self.tokens[self.pos - 1].1,
            ));
        }
        let line = self.line();
        Ok(Expr::Name(self.name()?, line))
    }
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "fn" | "requires"
            | "ensures"
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

/// Parses every function in `source`.
pub(crate) fn parse(source: &str) -> Result<Vec<Function>, Diagnostic> {
    let mut parser = Parser {
        source: source.lines().collect(),
        tokens: tokenize(source)?,
        pos: 0,
    };
    let mut functions = Vec::new();
    while parser.peek().is_some() {
        functions.push(parser.function()?);
    }
    Ok(functions)
}
