//! A machine's evidence. Its inits, events and observers are checked as
//! functions that read the machine's state, with its invariants assumed.
//! The solvers decide each obligation; where one is refuted from a state,
//! the explorer decides whether a run from an init reaches it. The machine
//! is proved only by the solvers, and refuted only by a replayed run.

use super::outcome_of;
use crate::explore::{self, Exploration as Explored, Member, Step, Trace};
use crate::interpret;
use crate::obligation::{self, Claim, Obligation};
use crate::syntax::{Machine, Program, Role};
use crate::{
    Definition, DefinitionKind, ExampleRun, Exploration, Inputs, ObligationKind, Outcome,
    Refutation, Run, Solvers, Standing, TraceStep, Verdict, identity,
};

/// What checking a machine established, short of its standing.
pub(super) struct Checked {
    pub verdicts: Vec<Verdict>,
    pub runs: Vec<ExampleRun>,
    pub definition: Definition,
    /// The functions of the program, not of the machine, that its members call.
    pub callees: Vec<String>,
}

/// The program as the machine's members see it: its functions, and the
/// machine's observers as functions known by their contracts, without the
/// invariants they assume.
fn callee_view(program: &Program, machine: &Machine) -> Program {
    Program {
        enums: program.enums.clone(),
        functions: machine.functions(&program.functions),
        machines: Vec::new(),
        examples: Vec::new(),
        machine_examples: Vec::new(),
    }
}

/// A run as steps in source terms: each call with its arguments, and the
/// state it leaves.
fn steps(units: &[identity::Unit], trace: &Trace, last: bool) -> Vec<TraceStep> {
    let step = |step: &Step| {
        let unit = &units[step.member];
        let function = unit.source;
        let state = function.machine.map_or(0, |m| m.state);
        let own = &function.params[state..];
        let args: Vec<String> = own
            .iter()
            .zip(&step.args)
            .map(|((name, _), value)| {
                if own.len() == 1 {
                    unit.source_of(value)
                } else {
                    format!("{name}: {}", unit.source_of(value))
                }
            })
            .collect();
        TraceStep {
            call: format!("{}({})", function.name, args.join(", ")),
            state: step.state.as_ref().map(|values| {
                function.params[..state]
                    .iter()
                    .zip(values)
                    .map(|((name, _), value)| (name.clone(), unit.source_of(value)))
                    .collect()
            }),
        }
    };
    trace
        .path
        .iter()
        .chain(last.then_some(&trace.last))
        .map(step)
        .collect()
}

/// The arguments of `step`, as inputs in source terms.
fn arguments(unit: &identity::Unit, step: &Step) -> Inputs {
    let function = unit.source;
    let state = function.machine.map_or(0, |m| m.state);
    function.params[state..]
        .iter()
        .zip(&step.args)
        .map(|((name, _), value)| (name.clone(), unit.source_of(value)))
        .collect()
}

/// Without the placeholders an init has for the state it doesn't read.
fn own_only(unit: &identity::Unit, inputs: Inputs) -> Inputs {
    let state = unit.source.machine.map_or(0, |m| m.state);
    let fields: Vec<&String> = unit.source.params[..state].iter().map(|(n, _)| n).collect();
    inputs
        .into_iter()
        .filter(|(name, _)| !fields.contains(&name))
        .collect()
}

fn unreached(explored: &Explored) -> String {
    format!(
        "no run from an init was found to a state that allows it: {} states explored, {}",
        explored.states,
        if explored.complete {
            "every one of them"
        } else {
            "but not every state, or every way to move from one, could be"
        }
    )
}

/// What deciding a machine's obligations draws on.
struct Context<'a> {
    units: &'a [identity::Unit<'a>],
    members: &'a [Member<'a>],
    explored: &'a Explored,
    solvers: &'a Solvers,
}

impl Context<'_> {
    fn note(&self, trace: Option<Vec<TraceStep>>) -> Option<Exploration> {
        Some(Exploration {
            states: self.explored.states,
            complete: self.explored.complete,
            trace,
        })
    }

    /// The outcome of the obligation `ob`, the `o`th of member `m`, whether it
    /// came from the ledger, and what exploring the states covered if it
    /// was consulted.
    fn decide(&self, m: usize, o: usize, ob: &Obligation) -> (Outcome, bool, Option<Exploration>) {
        let unit = &self.units[m];
        let role = unit.source.machine.map_or(Role::Observer, |x| x.role);
        let allowed = matches!(ob.claim, Claim::Allowed);
        if role == Role::Event
            && allowed
            && let Some(trace) = self
                .explored
                .allowed
                .get(&m)
                .filter(|trace| explore::confirms(self.members, m, None, trace))
        {
            let proved = Outcome::Proved {
                witness: Some(arguments(unit, &trace.last)),
            };
            return (
                proved,
                false,
                self.note(Some(steps(self.units, trace, true))),
            );
        }
        let (outcome, reused) = self.solvers.decide_calls(
            &ob.script,
            &unit.callee_bodies,
            &ob.params,
            ob.polarity,
            |model| interpret::replay(ob, model),
        );
        // An init that breaks an invariant is a run of one step.
        let started = match (&ob.claim, &outcome) {
            (
                Claim::Invariants,
                Outcome::Refuted {
                    evidence: Refutation::Counterexample(model),
                },
            ) if role == Role::Init => explore::start(self.members, m, model),
            _ => None,
        };
        let outcome = outcome_of(unit, ob, outcome);
        if let Some(last) = started {
            let trace = Trace {
                path: Vec::new(),
                last,
            };
            let refuted = Outcome::Refuted {
                evidence: Refutation::Trace(steps(self.units, &trace, true)),
            };
            return (refuted, false, None);
        }
        let from_state =
            role != Role::Init && !allowed && ob.polarity == obligation::Polarity::Universal;
        let failure = self
            .explored
            .failures
            .get(&(m, o))
            .filter(|trace| from_state && explore::confirms(self.members, m, Some(ob), trace));
        // A run that breaks it refutes it, whatever else the solvers say;
        // if they proved it, they proved it from the contract of a function
        // the run calls, or something is wrong.
        if let Some(trace) = failure {
            let outcome = match outcome {
                Outcome::Proved { .. } => Outcome::Undecided {
                    reason: "the solvers proved it, but a run from an init breaks it".to_string(),
                },
                _ => Outcome::Refuted {
                    evidence: Refutation::Trace(steps(self.units, trace, true)),
                },
            };
            return (outcome, false, self.note(None));
        }
        let outcome = match (role, outcome) {
            (Role::Event, Outcome::Proved { .. }) if allowed => Outcome::Undecided {
                reason: unreached(self.explored),
            },
            (Role::Init, Outcome::Proved { witness: Some(w) }) => Outcome::Proved {
                witness: Some(own_only(unit, w)),
            },
            (
                Role::Init,
                Outcome::Refuted {
                    evidence: Refutation::Counterexample(inputs),
                },
            ) => Outcome::Refuted {
                evidence: Refutation::Counterexample(own_only(unit, inputs)),
            },
            (
                _,
                Outcome::Refuted {
                    evidence: Refutation::Counterexample(inputs),
                },
            ) if from_state => Outcome::NotInductive { inputs },
            (_, other) => other,
        };
        let consulted =
            role == Role::Event && allowed || matches!(outcome, Outcome::NotInductive { .. });
        // What the explorer turned into another outcome isn't the ledger's.
        let reused = reused
            && !matches!(
                outcome,
                Outcome::NotInductive { .. } | Outcome::Undecided { .. }
            );
        (
            outcome,
            reused,
            consulted.then(|| self.note(None)).flatten(),
        )
    }
}

/// Checks `machine`: every obligation of its members and each of its
/// examples.
pub(super) fn check(program: &Program, machine: &Machine, solvers: &Solvers) -> Checked {
    let view = callee_view(program, machine);
    let units: Vec<identity::Unit> = machine
        .members()
        .map(|member| identity::unit(&view, member))
        .collect();
    let obligations: Vec<Vec<Obligation>> = units
        .iter()
        .map(|unit| obligation::unit_obligations(&unit.program))
        .collect();
    let members: Vec<Member> = units
        .iter()
        .zip(&obligations)
        .map(|(unit, obligations)| Member { unit, obligations })
        .collect();
    let explored = explore::explore(&members);
    let hash = identity::hash(
        &units
            .iter()
            .map(|unit| unit.hash.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );
    let context = Context {
        units: &units,
        members: &members,
        explored: &explored,
        solvers,
    };
    let mut verdicts = Vec::new();
    for (m, obligations) in obligations.iter().enumerate() {
        for (o, ob) in obligations.iter().enumerate() {
            let (outcome, reused, exploration) = context.decide(m, o, ob);
            verdicts.push(Verdict {
                kind: match ob.claim {
                    Claim::Ensures(_) => ObligationKind::Postcondition,
                    Claim::Covered => ObligationKind::Coverage,
                    Claim::Reaches(_) => ObligationKind::RowReachability,
                    Claim::InRange { .. } => ObligationKind::Overflow,
                    Claim::Requires { .. } => ObligationKind::Precondition,
                    Claim::InBounds { .. } => ObligationKind::Bounds,
                    Claim::Invariants => ObligationKind::Invariant,
                    Claim::Allowed => ObligationKind::Allowed,
                },
                outcome,
                function: machine.name.clone(),
                key: identity::hash(&ob.script),
                definition: hash.clone(),
                reused,
                clause: ob.clause.clone(),
                line: ob.line,
                exploration,
                parameters: units[m].source.params.clone(),
            });
        }
    }
    let functions: Vec<&str> = machine.observers.iter().map(|f| f.name.as_str()).collect();
    let mut callees: Vec<String> = Vec::new();
    for name in units.iter().flat_map(|unit| unit.callees.iter()) {
        if !functions.contains(&name.as_str()) && !callees.contains(name) {
            callees.push(name.clone());
        }
    }
    let runs = program
        .machine_examples
        .iter()
        .filter(|example| example.machine == machine.name)
        .map(|example| ExampleRun {
            function: machine.name.clone(),
            text: example.text.clone(),
            line: example.line,
            run: match interpret::run_machine(example, machine, &view.functions) {
                Ok(()) => Run::Passed,
                Err(reason) => Run::Failed { reason },
            },
        })
        .collect();
    let mut enums: Vec<(String, Vec<String>)> = Vec::new();
    for unit in &units {
        for e in unit.source_enums() {
            if !enums.contains(&e) {
                enums.push(e);
            }
        }
    }
    let definition = Definition {
        kind: DefinitionKind::Machine,
        name: machine.name.clone(),
        intent: machine.intent.clone(),
        line: machine.line,
        params: machine.state.clone(),
        result_type: None,
        specification: machine
            .invariants
            .iter()
            .filter(|c| !c.helper)
            .map(|c| (c.line, c.text.clone()))
            .collect(),
        support: machine
            .invariants
            .iter()
            .filter(|c| c.helper)
            .map(|c| (c.line, c.text.clone()))
            .collect(),
        enums,
        file: None,
        hash,
        standing: Standing::Unverified,
        unverified_callees: Vec::new(),
    };
    Checked {
        verdicts,
        runs,
        definition,
        callees,
    }
}
