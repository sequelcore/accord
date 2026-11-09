//! The syntax tree for the subset Accord compiles today: enums, and functions
//! over `Bool` and enums with `requires`, `ensures` and a body that is an
//! expression or a decision table.

mod parser;
mod token;

pub(crate) use parser::parse;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Type {
    Bool,
    /// An enum, by name.
    Enum(String),
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
    /// A parameter, `result`, or a bare variant until the checker resolves it.
    Name(String, usize),
    /// A variant of the named enum: written qualified, or resolved by the checker.
    Variant(String, String, usize),
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

/// One row of a decision table; `condition` is `None` for `otherwise`.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub condition: Option<Expr>,
    pub result: Expr,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum Body {
    Expr(Expr),
    /// A decision table, with the line of `decide`.
    Decide(Vec<Row>, usize),
}

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub name: String,
    pub line: usize,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub clauses: Vec<Clause>,
    pub body: Body,
}

#[derive(Clone, Debug)]
pub(crate) struct Enum {
    pub name: String,
    pub variants: Vec<String>,
    pub line: usize,
}

/// Everything declared in one file, in source order within each kind.
#[derive(Clone, Debug)]
pub(crate) struct Program {
    pub enums: Vec<Enum>,
    pub functions: Vec<Function>,
}
