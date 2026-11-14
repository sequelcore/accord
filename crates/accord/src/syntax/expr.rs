//! Expressions, by precedence: `implies`, `or`, `and`, `not`, comparison,
//! `+` and `-`, `*`, unary `-`, then literals, names, calls, records, `if`,
//! `match` and parentheses, each followed by any `[index]` or `.field`.

use super::parser::Parser;
use super::token::Token;
use super::{Arith, Arm, Binding, Expr, Op, Quantifier, Range};
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
        let op = if self.at_word("in") {
            Op::In
        } else if let Some(&(_, op)) = ops.iter().find(|(sym, _)| self.at_sym(sym)) {
            op
        } else {
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

    /// An atom, then any `[index]` or `.field` after it.
    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.pos;
        let mut expr = self.atom()?;
        loop {
            if self.at_sym("[") {
                self.pos += 1;
                let index = self.expr()?;
                self.expect_sym("]")?;
                expr = Expr::Index(Box::new(expr), Box::new(index), self.site(start));
            } else if self.at_sym(".") {
                let line = self.line();
                self.pos += 1;
                expr = Expr::Field(Box::new(expr), None, self.name()?, line);
            } else {
                return Ok(expr);
            }
        }
    }

    /// `match value { | Pattern => result ... }`.
    fn matched(&mut self) -> Result<Expr, Diagnostic> {
        let line = self.line();
        self.expect_word("match")?;
        let value = self.expr()?;
        self.expect_sym("{")?;
        let mut arms = Vec::new();
        while self.at_sym("|") {
            self.pos += 1;
            let arm_line = self.line();
            if self.at_word("_") {
                return self.error("a `match` has no catch-all: name every variant");
            }
            let variant = self.name()?;
            let bindings = if self.at_sym("(") {
                self.bindings()?
            } else {
                Vec::new()
            };
            self.expect_sym("=>")?;
            arms.push(Arm {
                variant,
                bindings,
                test: None,
                result: self.expr()?,
                line: arm_line,
            });
        }
        if arms.is_empty() {
            return self.error("expected a row starting with `|`");
        }
        self.expect_sym("}")?;
        Ok(Expr::Match(Box::new(value), arms, line))
    }

    /// A pattern's parenthesized names, each with its label where written;
    /// `_` names nothing.
    fn bindings(&mut self) -> Result<Vec<Binding>, Diagnostic> {
        self.expect_sym("(")?;
        let mut bindings = Vec::new();
        while !self.at_sym(")") {
            let labelled = matches!(self.tokens.get(self.pos + 1), Some((Token::Sym(":"), ..)));
            let label = if labelled {
                let label = self.name()?;
                self.expect_sym(":")?;
                Some(label)
            } else {
                None
            };
            let name = self.name()?;
            bindings.push((label, (name != "_").then_some(name)));
            if !self.at_sym(")") {
                self.expect_sym(",")?;
            }
        }
        self.expect_sym(")")?;
        Ok(bindings)
    }

    /// Whether a `{` here opens a record's value: one is followed by a field
    /// and `:`, which no block starts with.
    fn at_record(&self) -> bool {
        self.at_sym("{")
            && matches!(self.tokens.get(self.pos + 1), Some((Token::Word(_), ..)))
            && matches!(self.tokens.get(self.pos + 2), Some((Token::Sym(":"), ..)))
    }

    /// `{ field: value, ... }` after a record's name.
    fn record_value(&mut self, name: String, line: usize) -> Result<Expr, Diagnostic> {
        self.expect_sym("{")?;
        let mut fields = Vec::new();
        while !self.at_sym("}") {
            let field = self.name()?;
            self.expect_sym(":")?;
            fields.push((field, self.expr()?));
            if !self.at_sym("}") {
                self.expect_sym(",")?;
            }
        }
        self.expect_sym("}")?;
        Ok(Expr::Record(name, fields, line))
    }

    /// `all name in range: condition`, `any …` or `count …`: the condition
    /// extends as far as an expression does.
    fn quantified(&mut self, quantifier: Quantifier) -> Result<Expr, Diagnostic> {
        let line = self.line();
        self.pos += 1;
        let name = self.name()?;
        self.expect_word("in")?;
        let range = if self.at_word("indices") {
            self.pos += 1;
            self.expect_sym("(")?;
            let list = self.expr()?;
            self.expect_sym(")")?;
            Range::Indices(Box::new(list))
        } else {
            Range::Elements(Box::new(self.additive()?))
        };
        self.expect_sym(":")?;
        let condition = self.expr()?;
        Ok(Expr::Quantified(
            quantifier,
            name,
            range,
            Box::new(condition),
            line,
        ))
    }

    fn atom(&mut self) -> Result<Expr, Diagnostic> {
        for quantifier in [Quantifier::All, Quantifier::Any, Quantifier::Count] {
            if self.at_word(quantifier.name()) {
                return self.quantified(quantifier);
            }
        }
        if self.at_word("match") {
            return self.matched();
        }
        if self.at_word("length") {
            let line = self.line();
            self.pos += 1;
            self.expect_sym("(")?;
            let list = self.expr()?;
            self.expect_sym(")")?;
            return Ok(Expr::Length(Box::new(list), line));
        }
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
        if self.at_sym("[") {
            self.pos += 1;
            let mut items = Vec::new();
            while !self.at_sym("]") {
                items.push(self.expr()?);
                if !self.at_sym("]") {
                    self.expect_sym(",")?;
                }
            }
            self.expect_sym("]")?;
            return Ok(Expr::List(items, None, line));
        }
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
        let start = self.pos;
        let name = self.name()?;
        if self.at_sym(".") {
            // `Type.Variant`, or a value's field, which the checker tells
            // apart.
            self.pos += 1;
            let variant = self.name()?;
            if self.at_sym("(") {
                let args = self.arguments()?;
                return Ok(Expr::Construct(name, variant, args, line));
            }
            return Ok(Expr::Variant(name, variant, line));
        }
        if self.at_record() {
            return self.record_value(name, line);
        }
        if self.at_sym("(") {
            let args = self.arguments()?;
            return Ok(Expr::Call(name, args, self.site(start)));
        }
        Ok(Expr::Name(name, line))
    }
}
