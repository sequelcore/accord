//! Expressions, by precedence: `implies`, `or`, `and`, `not`, comparison,
//! `+` and `-`, `*`, unary `-`, then literals, names, `if` and parentheses.

use super::parser::Parser;
use super::token::Token;
use super::{Arith, Expr, Op};
use crate::Diagnostic;

impl Parser<'_> {
    pub(super) fn expr(&mut self) -> Result<Expr, Diagnostic> {
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
        let left = self.additive()?;
        let ops = [
            ("==", Op::Eq),
            ("!=", Op::Ne),
            ("<", Op::Lt),
            ("<=", Op::Le),
            (">", Op::Gt),
            (">=", Op::Ge),
        ];
        let Some(&(_, op)) = ops.iter().find(|(sym, _)| self.at_sym(sym)) else {
            return Ok(left);
        };
        self.pos += 1;
        Ok(Expr::Binary(op, Box::new(left), Box::new(self.additive()?)))
    }

    fn additive(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.pos;
        let mut left = self.multiplicative()?;
        loop {
            let op = if self.at_sym("+") {
                Arith::Add
            } else if self.at_sym("-") {
                Arith::Sub
            } else {
                return Ok(left);
            };
            self.pos += 1;
            let right = self.multiplicative()?;
            left = Expr::Arith(op, Box::new(left), Box::new(right), self.site(start));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.pos;
        let mut left = self.negation()?;
        while self.at_sym("*") {
            self.pos += 1;
            let right = self.negation()?;
            left = Expr::Arith(
                Arith::Mul,
                Box::new(left),
                Box::new(right),
                self.site(start),
            );
        }
        Ok(left)
    }

    fn negation(&mut self) -> Result<Expr, Diagnostic> {
        if !self.at_sym("-") {
            return self.primary();
        }
        let start = self.pos;
        self.pos += 1;
        let inner = self.negation()?;
        Ok(Expr::Neg(Box::new(inner), self.site(start)))
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
        let line = self.line();
        if let Some(Token::Str(text)) = self.peek() {
            let text = text.clone();
            self.pos += 1;
            return Ok(Expr::Text(text, line));
        }
        if let Some(Token::Word(word)) = self.peek()
            && word.starts_with(|c: char| c.is_ascii_digit())
        {
            let value = word
                .parse::<i128>()
                .ok()
                .filter(|_| word.chars().all(|c| c.is_ascii_digit()));
            return match value {
                Some(value) => {
                    self.pos += 1;
                    Ok(Expr::Int(value, line))
                }
                None => self.error(format!("`{word}` isn't an integer Accord can read")),
            };
        }
        if self.at_word("result") {
            self.pos += 1;
            return Ok(Expr::Name("result".to_string(), line));
        }
        let name = self.name()?;
        if self.at_sym(".") {
            self.pos += 1;
            return Ok(Expr::Variant(name, self.name()?, line));
        }
        Ok(Expr::Name(name, line))
    }
}
