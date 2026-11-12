//! The syntax tree for the subset Accord compiles today: enums, and functions
//! over `Bool`, fixed-width integers, enums, texts and lists with `requires`,
//! `ensures` and a body that is an expression or a decision table, which may
//! call other functions.

mod expr;
mod parser;
mod token;

pub(crate) use parser::parse;

/// A type supported by Accord's checked functions.
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    /// A Boolean.
    Bool,
    /// A signed fixed-width integer.
    Int(Width),
    /// A sequence of characters.
    Text,
    /// A list whose elements have this type.
    List(Box<Type>),
    /// An enum, by name.
    Enum(String),
}

/// A signed integer type's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Width {
    /// A signed 32-bit integer.
    I32,
    /// A signed 64-bit integer.
    I64,
}

impl Width {
    /// The Accord spelling.
    pub fn name(self) -> &'static str {
        match self {
            Width::I32 => "i32",
            Width::I64 => "i64",
        }
    }

    /// The smallest representable value.
    pub fn min(self) -> i128 {
        match self {
            Width::I32 => i128::from(i32::MIN),
            Width::I64 => i128::from(i64::MIN),
        }
    }

    /// The largest representable value.
    pub fn max(self) -> i128 {
        match self {
            Width::I32 => i128::from(i32::MAX),
            Width::I64 => i128::from(i64::MAX),
        }
    }
}

impl Type {
    /// The type as written in a function signature.
    pub fn name(&self) -> String {
        match self {
            Self::Bool => "Bool".to_string(),
            Self::Int(width) => width.name().to_string(),
            Self::Text => "Text".to_string(),
            Self::List(element) => format!("List<{}>", element.name()),
            Self::Enum(name) => name.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    And,
    Or,
    Implies,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// An element in a list.
    In,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Arith {
    Add,
    Sub,
    Mul,
}

/// Where an arithmetic operation is and what it is, for its overflow
/// obligation. The checker sets `width`; the parser leaves it `None`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Site {
    pub width: Option<Width>,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    Bool(bool),
    /// A non-negative integer literal; `-` before it is a negation.
    Int(i128, usize),
    /// A text literal's characters, without its quotes.
    Text(String, usize),
    /// A list literal, with its element type once the checker knows it.
    List(Vec<Expr>, Option<Type>, usize),
    /// A parameter, `result`, or a bare variant until the checker resolves it.
    Name(String, usize),
    /// A variant of the named enum: written qualified, or resolved by the checker.
    Variant(String, String, usize),
    Not(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Arith(Arith, Box<Expr>, Box<Expr>, Site),
    Neg(Box<Expr>, Site),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// A call: the function's name, each argument with its label where
    /// written, and where the call is. The checker puts the arguments in the
    /// order of the parameters, each labelled.
    Call(String, Vec<(Option<String>, Expr)>, Site),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClauseKind {
    Requires,
    Ensures,
    /// `open when`: inputs for which any result the contract allows is fine.
    Open,
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
    /// Declared `pub`: exported from its file's generated module.
    pub public: bool,
    pub intent: Option<String>,
    pub line: usize,
    /// The index of the file it was declared in, within its folder.
    pub file: usize,
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
    pub file: usize,
}

/// `example name(label: value, ...) == value`: a concrete call and the
/// result it must give. A label is `None` where the call omits it.
#[derive(Clone, Debug)]
pub(crate) struct Example {
    pub function: String,
    pub file: usize,
    pub args: Vec<(Option<String>, Expr)>,
    pub expected: Expr,
    pub text: String,
    pub line: usize,
}

/// Everything declared in one file, in source order within each kind.
#[derive(Clone, Debug)]
pub(crate) struct Program {
    pub enums: Vec<Enum>,
    pub functions: Vec<Function>,
    pub examples: Vec<Example>,
}
