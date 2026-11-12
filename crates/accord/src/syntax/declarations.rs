//! Declarations: types, enums, functions with their clauses and bodies,
//! decision tables, and examples.

use super::parser::Parser;
use super::token::Token;
use super::{Body, Clause, ClauseKind, Enum, Example, Function, Row, Type, Width};
use crate::Diagnostic;

impl Parser<'_> {
    fn ty(&mut self) -> Result<Type, Diagnostic> {
        let name = self.name()?;
        Ok(match name.as_str() {
            "Bool" => Type::Bool,
            "i32" => Type::Int(Width::I32),
            "i64" => Type::Int(Width::I64),
            "Text" => Type::Text,
            "List" => {
                self.expect_sym("<")?;
                let element = self.ty()?;
                self.expect_sym(">")?;
                Type::List(Box::new(element))
            }
            _ => Type::Enum(name),
        })
    }

    pub(super) fn enumeration(&mut self) -> Result<Enum, Diagnostic> {
        let line = self.line();
        self.expect_word("enum")?;
        let name = self.name()?;
        self.expect_sym("{")?;
        let (mut variants, mut values) = (Vec::new(), Vec::new());
        while !self.at_sym("}") {
            variants.push(self.name()?);
            values.push(if self.at_sym("=") {
                self.pos += 1;
                match self.peek().cloned() {
                    Some(Token::Str(text)) => {
                        self.pos += 1;
                        Some(text)
                    }
                    _ => return self.error("expected the variant's value as a string"),
                }
            } else {
                None
            });
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
            values,
            line,
            file: 0,
        })
    }

    pub(super) fn function(&mut self) -> Result<Function, Diagnostic> {
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
        let intent = if self.at_word("intent") {
            self.pos += 1;
            match self.peek().cloned() {
                Some(Token::Str(text)) => {
                    self.pos += 1;
                    Some(text)
                }
                _ => return self.error("expected the intent as a string"),
            }
        } else {
            None
        };
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
            public: false,
            intent,
            line,
            file: 0,
            params,
            ret,
            clauses,
            body,
        })
    }

    pub(super) fn example(&mut self) -> Result<Example, Diagnostic> {
        let (text, line) = self.rest_of_line();
        self.expect_word("example")?;
        let function = self.name()?;
        let args = self.arguments()?;
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
            file: 0,
            args,
            expected,
            text,
            line,
        })
    }

    /// A call's parenthesized arguments, each with its label where written.
    pub(super) fn arguments(&mut self) -> Result<Vec<(Option<String>, super::Expr)>, Diagnostic> {
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
        Ok(args)
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
