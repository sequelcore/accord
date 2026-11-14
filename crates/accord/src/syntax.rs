//! The syntax tree for the subset Accord compiles today: enums, whose
//! variants may carry fields, records, and functions over `Bool`,
//! fixed-width integers, enums, records, texts and lists with `requires`,
//! `ensures` and a body that is an expression or a decision table, which may
//! call other functions.

mod declarations;
mod expr;
mod machines;
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
    /// An enum, by name. The parser reads every name it doesn't know as an
    /// enum; the checker turns a record's name into [`Type::Record`].
    Enum(String),
    /// A record, by name.
    Record(String),
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
            Self::Enum(name) | Self::Record(name) => name.clone(),
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
    /// `all`, `any` or `count`: the name it binds, what that name ranges
    /// over, the condition, and its line.
    Quantified(Quantifier, String, Range, Box<Expr>, usize),
    /// `length(list)`, with its line.
    Length(Box<Expr>, usize),
    /// `list[index]`, and where it is.
    Index(Box<Expr>, Box<Expr>, Site),
    /// `next { field: value, ... }`, the state an init or event ends in, and
    /// its line. Only a machine's init or event ends in one; once checked, it
    /// gives every field of the state, in the state's order.
    Next(Vec<(String, Expr)>, usize),
    /// A variant with fields built like a call: its enum, which the checker
    /// sets where it is written bare, the variant, and each field with its
    /// label where written. The checker puts the fields in their declared
    /// order, each labelled.
    Construct(String, String, Vec<(Option<String>, Expr)>, usize),
    /// `Name { field: value, ... }`: a record's value. The checker puts the
    /// fields in their declared order.
    Record(String, Vec<(String, Expr)>, usize),
    /// `value.field`, with what the field belongs to once checked. A name a
    /// `match` arm binds becomes the field of the value matched, read only
    /// where the arm's variant is the value's.
    Field(Box<Expr>, Option<Owner>, String, usize),
    /// `match value { | Pattern => result ... }`, with its line.
    Match(Box<Expr>, Vec<Arm>, usize),
    /// Whether the value is the named variant of the named enum. Only the
    /// checker writes one: the condition under which a `match` arm runs.
    Is(Box<Expr>, String, String),
}

/// What a field belongs to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Owner {
    /// The named record.
    Record(String),
    /// The named enum's named variant.
    Variant(String, String),
}

/// A pattern's name for one field, with the field's label where written;
/// the name is `None` for `_`.
pub(crate) type Binding = (Option<String>, Option<String>);

/// One row of a `match`: a variant, a name for each of its fields or `_`
/// for none, and the result. The checker puts the names in the fields'
/// declared order and sets `test`, the condition under which the arm runs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arm {
    pub variant: String,
    pub bindings: Vec<Binding>,
    pub test: Option<Expr>,
    pub result: Expr,
    pub line: usize,
}

impl Expr {
    /// The line the expression starts on, or 0 for a `Bool` literal, which
    /// carries none.
    pub fn line(&self) -> usize {
        match self {
            Expr::Name(_, line)
            | Expr::Variant(_, _, line)
            | Expr::Int(_, line)
            | Expr::Text(_, line)
            | Expr::List(_, _, line)
            | Expr::Quantified(.., line)
            | Expr::Next(_, line)
            | Expr::Construct(.., line)
            | Expr::Record(_, _, line)
            | Expr::Field(.., line)
            | Expr::Match(_, _, line)
            | Expr::Length(_, line) => *line,
            Expr::Arith(.., site)
            | Expr::Neg(_, site)
            | Expr::Call(.., site)
            | Expr::Index(.., site) => site.line,
            Expr::Not(inner) | Expr::Is(inner, ..) => inner.line(),
            Expr::Binary(_, left, _) | Expr::If(left, _, _) => left.line(),
            Expr::Bool(_) => 0,
        }
    }

    /// The expressions directly inside this one.
    pub fn children(&self) -> Vec<&Expr> {
        match self {
            Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {
                Vec::new()
            }
            Expr::List(items, ..) => items.iter().collect(),
            Expr::Not(inner)
            | Expr::Neg(inner, _)
            | Expr::Length(inner, _)
            | Expr::Field(inner, ..)
            | Expr::Is(inner, ..) => vec![inner],
            Expr::Binary(_, left, right)
            | Expr::Arith(_, left, right, _)
            | Expr::Index(left, right, _) => vec![left, right],
            Expr::If(condition, then, otherwise) => vec![condition, then, otherwise],
            Expr::Call(_, args, _) | Expr::Construct(_, _, args, _) => {
                args.iter().map(|(_, arg)| arg).collect()
            }
            Expr::Quantified(_, _, range, condition, _) => vec![range.list(), condition],
            Expr::Next(fields, _) | Expr::Record(_, fields, _) => {
                fields.iter().map(|(_, value)| value).collect()
            }
            Expr::Match(value, arms, _) => {
                let mut out: Vec<&Expr> = vec![value];
                for arm in arms {
                    out.extend(arm.test.as_ref());
                    out.push(&arm.result);
                }
                out
            }
        }
    }

    /// The expressions directly inside this one, to change them.
    pub fn children_mut(&mut self) -> Vec<&mut Expr> {
        match self {
            Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) | Expr::Variant(..) => {
                Vec::new()
            }
            Expr::List(items, ..) => items.iter_mut().collect(),
            Expr::Not(inner)
            | Expr::Neg(inner, _)
            | Expr::Length(inner, _)
            | Expr::Field(inner, ..)
            | Expr::Is(inner, ..) => vec![inner],
            Expr::Binary(_, left, right)
            | Expr::Arith(_, left, right, _)
            | Expr::Index(left, right, _) => vec![left, right],
            Expr::If(condition, then, otherwise) => vec![condition, then, otherwise],
            Expr::Call(_, args, _) | Expr::Construct(_, _, args, _) => {
                args.iter_mut().map(|(_, arg)| arg).collect()
            }
            Expr::Quantified(_, _, range, condition, _) => vec![range.list_mut(), condition],
            Expr::Next(fields, _) | Expr::Record(_, fields, _) => {
                fields.iter_mut().map(|(_, value)| value).collect()
            }
            Expr::Match(value, arms, _) => {
                let mut out: Vec<&mut Expr> = vec![value];
                for arm in arms {
                    out.extend(arm.test.as_mut());
                    out.push(&mut arm.result);
                }
                out
            }
        }
    }
}

/// What a quantifier over a list gives: whether the condition holds of every
/// element, of some element, or of how many.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Quantifier {
    All,
    Any,
    Count,
}

impl Quantifier {
    /// The quantifier as written.
    pub fn name(self) -> &'static str {
        match self {
            Quantifier::All => "all",
            Quantifier::Any => "any",
            Quantifier::Count => "count",
        }
    }
}

/// What a quantifier's name ranges over: a list's elements, or its indices.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Range {
    Elements(Box<Expr>),
    Indices(Box<Expr>),
}

impl Range {
    /// The list the name ranges over.
    pub fn list(&self) -> &Expr {
        match self {
            Range::Elements(list) | Range::Indices(list) => list,
        }
    }

    /// The list the name ranges over, to change it.
    pub fn list_mut(&mut self) -> &mut Expr {
        match self {
            Range::Elements(list) | Range::Indices(list) => list,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClauseKind {
    Requires,
    Ensures,
    /// `open when`: inputs for which any result the contract allows is fine.
    Open,
    /// A machine's invariant or helper invariant. An event or observer
    /// assumes it of the state it reads; an init or event must meet it in
    /// the state it ends in.
    Invariant,
}

#[derive(Clone, Debug)]
pub(crate) struct Clause {
    pub kind: ClauseKind,
    /// A helper invariant: assumed and proved as an invariant is, but support
    /// for proving the others rather than a claim.
    pub helper: bool,
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

/// What a function is to the machine that declares it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Role {
    /// A way to start. Its body ends in a `next`; it doesn't read the state.
    Init,
    /// A transition. Its body ends in a `next`.
    Event,
    /// A pure function of the state.
    Observer,
}

/// A function that belongs to a machine. Once checked, its first `state`
/// parameters are the machine's fields, in order, and its invariants are
/// clauses. An init's fields are placeholders: it doesn't read them. An init
/// or event's `ret` is a placeholder too: its value is its `next`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Member {
    pub role: Role,
    pub state: usize,
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
    pub machine: Option<Member>,
}

/// A state, the ways it starts, the events that move it, and what must hold
/// in every state it reaches.
#[derive(Clone, Debug)]
pub(crate) struct Machine {
    pub name: String,
    pub public: bool,
    pub intent: Option<String>,
    pub line: usize,
    pub file: usize,
    pub state: Vec<(String, Type)>,
    pub inits: Vec<Function>,
    pub events: Vec<Function>,
    pub observers: Vec<Function>,
    /// Each invariant and helper invariant, as written.
    pub invariants: Vec<Clause>,
}

impl Machine {
    /// The functions a member of this machine may call: `globals`, and the
    /// machine's observers as callees know them, without the invariants they
    /// assume of the state they read.
    pub fn functions(&self, globals: &[Function]) -> Vec<Function> {
        let mut functions = globals.to_vec();
        for observer in &self.observers {
            let mut callee = observer.clone();
            callee.clauses.retain(|c| c.kind != ClauseKind::Invariant);
            functions.push(callee);
        }
        functions
    }

    /// Inits, then events, then observers: the order of the machine's
    /// obligations.
    pub fn members(&self) -> impl Iterator<Item = &Function> {
        self.inits.iter().chain(&self.events).chain(&self.observers)
    }
}

/// `example Machine`, then an init and events one per line, then
/// `gives condition`: a run and the condition that must hold after it.
#[derive(Clone, Debug)]
pub(crate) struct MachineExample {
    pub machine: String,
    pub file: usize,
    /// Each step as a call, which the checker puts in the order of the
    /// parameters, each labelled.
    pub steps: Vec<Expr>,
    pub gives: Expr,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Enum {
    pub name: String,
    pub variants: Vec<String>,
    /// Each variant's value outside Accord, where written: `Cancelled = "CANCELLED"`.
    pub values: Vec<Option<String>>,
    /// Each variant's fields, in order; empty for a variant without any.
    pub fields: Vec<Vec<(String, Type)>>,
    pub line: usize,
    pub file: usize,
}

impl Enum {
    /// The fields of the named variant.
    pub fn fields_of(&self, variant: &str) -> &[(String, Type)] {
        self.variants
            .iter()
            .position(|v| v == variant)
            .map_or(&[], |index| &self.fields[index])
    }

    /// Whether some variant carries fields.
    pub fn has_fields(&self) -> bool {
        self.fields.iter().any(|f| !f.is_empty())
    }
}

/// `record Name { field: Type, ... }`: a value made of named fields.
#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub line: usize,
    pub file: usize,
}

impl Enum {
    /// The string a variant is outside Accord, as in TypeScript: its written
    /// value, or else its `PascalCase` name in `snake_case`.
    pub fn text(&self, variant: &str) -> String {
        let index = self.variants.iter().position(|v| v == variant);
        if let Some(Some(value)) = index.map(|i| &self.values[i]) {
            return value.clone();
        }
        let mut out = String::new();
        for (index, c) in variant.chars().enumerate() {
            if c.is_uppercase() && index > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        }
        out
    }
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
    pub records: Vec<Record>,
    pub functions: Vec<Function>,
    pub machines: Vec<Machine>,
    pub examples: Vec<Example>,
    pub machine_examples: Vec<MachineExample>,
}
