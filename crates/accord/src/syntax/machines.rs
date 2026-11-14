//! Machines: a state, inits, events, observers and invariants, and the run
//! an example gives.

use super::parser::Parser;
use super::{
    Body, Clause, ClauseKind, Expr, Function, Machine, MachineExample, Member, Role, Type,
};
use crate::Diagnostic;

impl Parser<'_> {
    pub(super) fn machine(&mut self) -> Result<Machine, Diagnostic> {
        let line = self.line();
        self.expect_word("machine")?;
        let name = self.name()?;
        self.expect_sym("{")?;
        let intent = self.intent()?;
        let mut machine = Machine {
            name,
            public: false,
            intent,
            line,
            file: 0,
            state: Vec::new(),
            inits: Vec::new(),
            events: Vec::new(),
            observers: Vec::new(),
            invariants: Vec::new(),
        };
        let mut stated = false;
        let mut names: Vec<String> = Vec::new();
        while !self.at_sym("}") {
            if self.at_word("state") {
                if stated {
                    return self.error("a machine has one `state`");
                }
                stated = true;
                machine.state = self.state()?;
            } else if self.at_word("init") {
                machine.inits.push(self.transition(Role::Init)?);
            } else if self.at_word("event") {
                machine.events.push(self.transition(Role::Event)?);
            } else if self.at_word("fn") {
                let mut observer = self.function()?;
                observer.machine = Some(Member {
                    role: Role::Observer,
                    state: 0,
                });
                machine.observers.push(observer);
            } else if self.at_word("invariant") || self.at_word("helper") {
                let line = self.line();
                let (name, invariant) = self.invariant()?;
                if names.contains(&name) {
                    return Err(Diagnostic::new(
                        line,
                        format!("`{name}` names two invariants of `{}`", machine.name),
                    ));
                }
                names.push(name);
                machine.invariants.push(invariant);
            } else {
                return self.error(
                    "expected `state`, `init`, `event`, `fn`, `invariant` or `helper invariant`",
                );
            }
        }
        self.expect_sym("}")?;
        if !stated {
            return Err(Diagnostic::new(
                line,
                format!("`{}` has no `state`", machine.name),
            ));
        }
        Ok(machine)
    }

    /// `state { field: Type, ... }`.
    fn state(&mut self) -> Result<Vec<(String, Type)>, Diagnostic> {
        self.expect_word("state")?;
        self.fields()
    }

    /// An init or event: its name and parameters, `intent` and `requires`,
    /// and a body that ends in `next`.
    fn transition(&mut self, role: Role) -> Result<Function, Diagnostic> {
        let line = self.line();
        self.pos += 1;
        let name = self.name()?;
        let params = self.parameters()?;
        let intent = self.intent()?;
        let clauses = self.clauses(&["requires"])?;
        if self.at_word("ensures") || self.at_word("open") {
            return self.error("an init or event takes `intent` and `requires` only");
        }
        let body = if self.at_word("decide") {
            self.decide(Self::step)?
        } else {
            self.expect_sym("{")?;
            let body = self.step()?;
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
            ret: Type::Bool,
            clauses,
            body,
            machine: Some(Member { role, state: 0 }),
        })
    }

    /// `next { ... }`, or an `if` and its `else` that each end in one.
    pub(super) fn step(&mut self) -> Result<Expr, Diagnostic> {
        if self.at_word("if") {
            self.pos += 1;
            let condition = self.expr()?;
            self.expect_sym("{")?;
            let then = self.step()?;
            self.expect_sym("}")?;
            self.expect_word("else")?;
            self.expect_sym("{")?;
            let otherwise = self.step()?;
            self.expect_sym("}")?;
            return Ok(Expr::If(
                Box::new(condition),
                Box::new(then),
                Box::new(otherwise),
            ));
        }
        if !self.at_word("next") {
            return self.error("expected `next { … }` or an `if` that ends in one");
        }
        let line = self.line();
        self.pos += 1;
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
        Ok(Expr::Next(fields, line))
    }

    /// `invariant name: condition` or `helper invariant name: condition`.
    fn invariant(&mut self) -> Result<(String, Clause), Diagnostic> {
        let (text, line) = self.rest_of_line();
        let helper = self.at_word("helper");
        if helper {
            self.pos += 1;
        }
        self.expect_word("invariant")?;
        let name = self.name()?;
        self.expect_sym(":")?;
        let clause = Clause {
            kind: ClauseKind::Invariant,
            helper,
            expr: self.expr()?,
            text,
            line,
        };
        Ok((name, clause))
    }

    /// `example Machine`, its steps one per line, then `gives condition`.
    pub(super) fn machine_example(&mut self) -> Result<MachineExample, Diagnostic> {
        let (_, line) = self.rest_of_line();
        self.expect_word("example")?;
        let machine = self.name()?;
        let mut steps = Vec::new();
        while !self.at_word("gives") {
            steps.push(self.expr()?);
        }
        self.pos += 1;
        let gives = self.expr()?;
        let last = self.tokens[self.pos - 1].1;
        let text = self.source[line - 1..last]
            .iter()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(MachineExample {
            machine,
            file: 0,
            steps,
            gives,
            text,
            line,
        })
    }
}
