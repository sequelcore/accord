//! Exploring a machine's reachable states, breadth first from every init, so
//! that the traces it finds are the shortest.
//!
//! A trace is what refutes an invariant, or an arithmetic operation, in a
//! state the solvers only reach from outside the machine, and what shows an
//! event is allowed in a state the machine reaches. The explorer runs the
//! machine with the interpreter, in code, and every trace is run again from its
//! init before it is believed ([`replay`]).
//!
//! A parameter is enumerated only when its values are finite and known:
//! `Bool`, an enum, or an integer whose `requires` lists its values or bounds
//! it by literals to at most [`MAX_DOMAIN`] values. An init or event with any
//! other parameter isn't explored, and exploration is then incomplete, as it
//! is when it reaches [`MAX_STATES`] states or runs [`MAX_STEPS`] steps.

use std::collections::{HashMap, VecDeque};

use crate::identity::Unit;
use crate::interpret::{self, Value};
use crate::obligation::{Claim, Obligation, Polarity};
use crate::syntax::{ClauseKind, Expr, Function, Op, Role, Type, Width};

/// The most states the explorer keeps.
const MAX_STATES: usize = 100_000;
/// The most times it runs an init, an event or an observer.
const MAX_STEPS: usize = 2_000_000;
/// The most values it enumerates for one integer parameter.
const MAX_DOMAIN: i128 = 1024;

/// An init, event or observer, normalized, with its obligations.
pub(crate) struct Member<'a> {
    pub unit: &'a Unit<'a>,
    pub obligations: &'a [Obligation<'a>],
}

impl Member<'_> {
    /// The normalized function, the first of the unit's.
    pub fn function(&self) -> &Function {
        &self.unit.program.functions[0]
    }

    fn role(&self) -> Role {
        self.function()
            .machine
            .map_or(Role::Observer, |member| member.role)
    }

    /// The names and values of the member's own parameters.
    fn own(&self, args: &[Value]) -> Vec<(String, Value)> {
        let function = self.function();
        let skip = function.machine.map_or(0, |m| m.state);
        function.params[skip..]
            .iter()
            .map(|(name, _)| name.clone())
            .zip(args.iter().cloned())
            .collect()
    }

    /// The values the member runs on in `state`: its fields, then `args`.
    pub fn names(&self, state: Option<&[Value]>, args: &[Value]) -> Vec<(String, Value)> {
        let mut names = state.map_or_else(Vec::new, |s| interpret::fields(self.function(), s));
        names.extend(self.own(args));
        names
    }

    /// The state the member ends in, when it is run in code.
    pub fn run(&self, state: Option<&[Value]>, args: &[Value]) -> Result<Vec<Value>, String> {
        let function = self.function();
        let skip = function.machine.map_or(0, |m| m.state);
        for ((name, ty), arg) in function.params[skip..].iter().zip(args) {
            if let (Type::Int(width), Value::Int(n)) = (ty, arg)
                && !(width.min()..=width.max()).contains(n)
            {
                return Err(format!("`{name}` is {n}, which is outside its type"));
            }
        }
        interpret::transition(
            self.function(),
            &self.names(state, args),
            &self.unit.program.functions,
        )
    }
}

/// One step of a run: an init or event, its arguments, and the state it
/// leaves, where it leaves one.
#[derive(Clone)]
pub(crate) struct Step {
    pub member: usize,
    pub args: Vec<Value>,
    pub state: Option<Vec<Value>>,
}

/// A run to a state, and what is done in that state: an event or observer,
/// with its arguments.
pub(crate) struct Trace {
    pub path: Vec<Step>,
    pub last: Step,
}

/// What exploring a machine found.
pub(crate) struct Exploration {
    /// The states reached.
    pub states: usize,
    /// Whether every state, and every way to move from it, was visited.
    pub complete: bool,
    /// For a member's obligation, by their positions, the shortest trace to a
    /// state where the obligation fails.
    pub failures: HashMap<(usize, usize), Trace>,
    /// For an event, by its position, the shortest trace to a state where its
    /// `requires` hold, with the arguments that make them.
    pub allowed: HashMap<usize, Trace>,
}

struct Node {
    state: Vec<Value>,
    via: (Option<usize>, usize, Vec<Value>),
}

fn path(nodes: &[Node], mut at: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    loop {
        let node = &nodes[at];
        steps.push(Step {
            member: node.via.1,
            args: node.via.2.clone(),
            state: Some(node.state.clone()),
        });
        match node.via.0 {
            Some(parent) => at = parent,
            None => break,
        }
    }
    steps.reverse();
    steps
}

/// The literal an expression is, if it is one.
fn literal(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Int(n, _) => Some(*n),
        Expr::Neg(inner, _) => literal(inner).map(|n| -n),
        _ => None,
    }
}

fn is(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Name(n, _) if n == name)
}

/// The values a condition lists for `name`: equalities to literals, joined by
/// `or`, or membership in a list of literals.
fn listed(name: &str, condition: &Expr) -> Option<Vec<i128>> {
    match condition {
        Expr::Binary(Op::Or, left, right) => {
            let mut values = listed(name, left)?;
            values.extend(listed(name, right)?);
            Some(values)
        }
        Expr::Binary(Op::Eq, left, right) if is(left, name) => literal(right).map(|n| vec![n]),
        Expr::Binary(Op::Eq, left, right) if is(right, name) => literal(left).map(|n| vec![n]),
        Expr::Binary(Op::In, element, list) if is(element, name) => match &**list {
            Expr::List(items, ..) => items.iter().map(literal).collect(),
            _ => None,
        },
        _ => None,
    }
}

fn conjuncts<'a>(condition: &'a Expr, out: &mut Vec<&'a Expr>) {
    match condition {
        Expr::Binary(Op::And, left, right) => {
            conjuncts(left, out);
            conjuncts(right, out);
        }
        other => out.push(other),
    }
}

/// The operator that reads the same with its sides swapped.
fn flipped(op: Op) -> Option<Op> {
    Some(match op {
        Op::Lt => Op::Gt,
        Op::Le => Op::Ge,
        Op::Gt => Op::Lt,
        Op::Ge => Op::Le,
        _ => return None,
    })
}

/// The values of an integer parameter that its `requires` pin down: those it
/// lists, or the range its literal bounds give, if that holds at most
/// [`MAX_DOMAIN`] values.
fn integers(name: &str, width: Width, requires: &[&Expr]) -> Option<Vec<Value>> {
    let (mut low, mut high): (Option<i128>, Option<i128>) = (None, None);
    let mut set: Option<Vec<i128>> = None;
    let mut conditions = Vec::new();
    requires.iter().for_each(|r| conjuncts(r, &mut conditions));
    for condition in conditions {
        if let Some(values) = listed(name, condition) {
            set = Some(match set {
                Some(old) => old.into_iter().filter(|v| values.contains(v)).collect(),
                None => values,
            });
            continue;
        }
        let Expr::Binary(op, left, right) = condition else {
            continue;
        };
        let bound = if is(left, name) {
            literal(right).map(|n| (*op, n))
        } else if is(right, name) {
            literal(left).and_then(|n| flipped(*op).map(|op| (op, n)))
        } else {
            None
        };
        match bound {
            Some((Op::Ge, n)) => low = Some(low.map_or(n, |l| l.max(n))),
            Some((Op::Gt, n)) => low = Some(low.map_or(n + 1, |l| l.max(n + 1))),
            Some((Op::Le, n)) => high = Some(high.map_or(n, |h| h.min(n))),
            Some((Op::Lt, n)) => high = Some(high.map_or(n - 1, |h| h.min(n - 1))),
            _ => {}
        }
    }
    let low = low.map_or(width.min(), |l| l.max(width.min()));
    let high = high.map_or(width.max(), |h| h.min(width.max()));
    let mut values: Vec<i128> = match (set, low > width.min() || high < width.max()) {
        (Some(set), _) => set
            .into_iter()
            .filter(|v| (low..=high).contains(v))
            .collect(),
        (None, true) if high.saturating_sub(low) < MAX_DOMAIN => (low..=high).collect(),
        _ => return None,
    };
    values.sort_unstable();
    values.dedup();
    Some(values.into_iter().map(Value::Int).collect())
}

/// Every combination of arguments the member is explored with, when the
/// values of each of its own parameters are known.
fn instances(member: &Member) -> Option<Vec<Vec<Value>>> {
    let function = member.function();
    let skip = function.machine.map_or(0, |m| m.state);
    let requires: Vec<&Expr> = function
        .clauses
        .iter()
        .filter(|c| c.kind == ClauseKind::Requires)
        .map(|c| &c.expr)
        .collect();
    let mut combinations: Vec<Vec<Value>> = vec![Vec::new()];
    for (name, ty) in &function.params[skip..] {
        let domain = match ty {
            Type::Bool => vec![Value::Bool(false), Value::Bool(true)],
            Type::Enum(owner) => {
                let e = member
                    .unit
                    .program
                    .enums
                    .iter()
                    .find(|e| e.name == *owner)?;
                e.variants
                    .iter()
                    .map(|v| Value::Variant(owner.clone(), v.clone()))
                    .collect()
            }
            Type::Int(width) => integers(name, *width, &requires)?,
            Type::Text | Type::List(_) => return None,
        };
        if combinations.len().checked_mul(domain.len())? > MAX_STEPS {
            return None;
        }
        combinations = combinations
            .into_iter()
            .flat_map(|prefix| {
                domain.iter().map(move |value| {
                    let mut args = prefix.clone();
                    args.push(value.clone());
                    args
                })
            })
            .collect();
    }
    Some(combinations)
}

/// Whether the member's `requires` hold on `names`.
fn admits(member: &Member, names: &[(String, Value)]) -> bool {
    matches!(
        interpret::broken_requires(member.function(), names),
        Ok(None)
    )
}

/// Explores the states `members` reach: its inits, events and observers, in
/// that order.
pub(crate) fn explore(members: &[Member]) -> Exploration {
    let domains: Vec<Option<Vec<Vec<Value>>>> = members.iter().map(instances).collect();
    let mut complete = members
        .iter()
        .zip(&domains)
        .all(|(member, domain)| member.role() == Role::Observer || domain.is_some());
    let mut nodes: Vec<Node> = Vec::new();
    let mut seen: HashMap<Vec<Value>, usize> = HashMap::new();
    let mut found = Exploration {
        states: 0,
        complete: false,
        failures: HashMap::new(),
        allowed: HashMap::new(),
    };
    let mut steps = 0usize;
    let mut add = |nodes: &mut Vec<Node>, state: Vec<Value>, via, complete: &mut bool| {
        if seen.contains_key(&state) {
            return;
        }
        if nodes.len() >= MAX_STATES {
            *complete = false;
            return;
        }
        seen.insert(state.clone(), nodes.len());
        nodes.push(Node { state, via });
    };
    for (m, member) in members.iter().enumerate() {
        let (Role::Init, Some(instances)) = (member.role(), &domains[m]) else {
            continue;
        };
        for args in instances {
            steps += 1;
            if let Ok(state) = member.run(None, args) {
                add(&mut nodes, state, (None, m, args.clone()), &mut complete);
            }
        }
    }
    let mut queue: VecDeque<usize> = (0..nodes.len()).collect();
    'states: while let Some(at) = queue.pop_front() {
        let state = nodes[at].state.clone();
        for (m, member) in members.iter().enumerate() {
            let role = member.role();
            let (Role::Event | Role::Observer, Some(instances)) = (role, &domains[m]) else {
                continue;
            };
            let meets = matches!(
                interpret::broken_invariant(member.function(), &state),
                Ok(None)
            );
            for args in instances {
                if steps >= MAX_STEPS {
                    complete = false;
                    break 'states;
                }
                steps += 1;
                let names = member.names(Some(&state), args);
                if !admits(member, &names) {
                    continue;
                }
                let next = (role == Role::Event)
                    .then(|| member.run(Some(&state), args).ok())
                    .flatten();
                let here = Step {
                    member: m,
                    args: args.clone(),
                    state: next.clone(),
                };
                if role == Role::Event {
                    found.allowed.entry(m).or_insert_with(|| Trace {
                        path: path(&nodes, at),
                        last: here.clone(),
                    });
                }
                if meets {
                    for (o, obligation) in member.obligations.iter().enumerate() {
                        if obligation.polarity == Polarity::Universal
                            && obligation.bound.is_empty()
                            && !matches!(obligation.claim, Claim::Allowed)
                            && !found.failures.contains_key(&(m, o))
                            && matches!(interpret::fails(obligation, &names), Ok(true))
                        {
                            found.failures.insert(
                                (m, o),
                                Trace {
                                    path: path(&nodes, at),
                                    last: here.clone(),
                                },
                            );
                        }
                    }
                }
                if let Some(next) = next {
                    let before = nodes.len();
                    add(&mut nodes, next, (Some(at), m, args.clone()), &mut complete);
                    if nodes.len() > before {
                        queue.push_back(nodes.len() - 1);
                    }
                }
            }
        }
    }
    found.states = nodes.len();
    found.complete = complete;
    found
}

/// The run that is only the init `m` called with the arguments a solver's
/// `model` gives it, provided the model admits them.
pub(crate) fn start(members: &[Member], m: usize, model: &[(String, String)]) -> Option<Step> {
    let member = &members[m];
    let function = member.function();
    let names = interpret::admitted(function, &model.to_vec()).ok()?;
    let args: Vec<Value> = names[function.machine?.state..]
        .iter()
        .map(|(_, value)| value.clone())
        .collect();
    let state = Some(member.run(None, &args).ok()?);
    Some(Step {
        member: m,
        args,
        state,
    })
}

/// Whether `trace`, run again from its init, reaches a state where the
/// member `m`'s `requires` hold on the trace's arguments and, for
/// `obligation`, where the state meets the invariants and the obligation
/// fails.
pub(crate) fn confirms(
    members: &[Member],
    m: usize,
    obligation: Option<&Obligation>,
    trace: &Trace,
) -> bool {
    let Ok(state) = replay(members, &trace.path) else {
        return false;
    };
    let member = &members[m];
    let names = member.names(Some(&state), &trace.last.args);
    admits(member, &names)
        && obligation.is_none_or(|obligation| {
            matches!(
                interpret::broken_invariant(member.function(), &state),
                Ok(None)
            ) && matches!(interpret::fails(obligation, &names), Ok(true))
        })
}

/// Runs `path` again from its init, and gives the state it ends in, or why it
/// doesn't end in the states it claims.
pub(crate) fn replay(members: &[Member], path: &[Step]) -> Result<Vec<Value>, String> {
    let mut state: Option<Vec<Value>> = None;
    for step in path {
        let member = &members[step.member];
        if (member.role() == Role::Init) != state.is_none() {
            return Err("a run starts with an init and goes on with events".to_string());
        }
        let next = member.run(state.as_deref(), &step.args)?;
        if step.state.as_ref() != Some(&next) {
            return Err("the run doesn't reach the state it claims".to_string());
        }
        state = Some(next);
    }
    state.ok_or_else(|| "the run is empty".to_string())
}
