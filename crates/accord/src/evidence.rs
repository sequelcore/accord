//! Everything one check establishes: each obligation's outcome, each
//! example's run, and the standing they add up to.

mod standing;

use standing::{file_standing, standing_of};

use crate::{
    Definition, Determinacy, ExampleRun, Inputs, ObligationKind, Outcome, Refutation, Report, Run,
    Solvers, Standing, Verdict, identity, interpret, mutation, obligation, openness, syntax,
};

/// What the solvers established about `o`, in source terms: a model's call
/// values are kept only to show what a weak callee contract allowed.
fn outcome_of(unit: &identity::Unit, o: &obligation::Obligation, outcome: Outcome) -> Outcome {
    let is_param = |name: &str| o.function.params.iter().any(|(p, _)| p == name);
    let params = |inputs: &Inputs| -> Inputs {
        inputs
            .iter()
            .filter(|(name, _)| is_param(name))
            .cloned()
            .collect()
    };
    let outcome = match outcome {
        Outcome::Proved {
            witness: Some(inputs),
        } => Outcome::Proved {
            witness: Some(params(&inputs)),
        },
        Outcome::Refuted {
            evidence: Refutation::Counterexample(inputs),
        } => Outcome::Refuted {
            evidence: Refutation::Counterexample(params(&inputs)),
        },
        Outcome::WeakCallee { inputs, .. } => {
            let calls = o
                .calls
                .iter()
                .filter_map(|(constant, call)| {
                    let syntax::Expr::Call(name, _, site) = call else {
                        return None;
                    };
                    let value = inputs.iter().find(|(n, _)| n == constant)?.1.clone();
                    let callee = o.functions.iter().find(|f| f.name == *name)?;
                    Some((
                        site.text.clone(),
                        unit.source_typed_value(&value, &callee.ret),
                    ))
                })
                .collect();
            Outcome::WeakCallee {
                inputs: params(&inputs),
                calls,
            }
        }
        other => other,
    };
    unit.source_outcome(outcome)
}

/// Everything a check establishes about a checked program's functions: all
/// of them, or only those declared in file `target`. The functions they call
/// are checked too, since a caller is verified only when its callees are,
/// but only the target's are reported.
pub(crate) fn report(
    program: &syntax::Program,
    target: Option<usize>,
    names: &[String],
    solvers: &Solvers,
) -> Report {
    let file_of = |file: usize| names.get(file).cloned();
    let all_units = identity::units(program);
    let targeted = |unit: &identity::Unit| target.is_none_or(|t| unit.source.file == t);
    let needed: Vec<String> = all_units
        .iter()
        .filter(|unit| targeted(unit))
        .flat_map(|unit| {
            std::iter::once(unit.source.name.clone()).chain(unit.callees.iter().cloned())
        })
        .collect();
    let units: Vec<_> = all_units
        .into_iter()
        .filter(|unit| needed.contains(&unit.source.name))
        .collect();
    let mut verdicts = Vec::new();
    for unit in &units {
        for o in obligation::unit_obligations(&unit.program) {
            let (outcome, reused) = solvers.decide_calls(
                &o.script,
                &unit.callee_bodies,
                &o.params,
                o.polarity,
                |model| interpret::replay(&o, model),
            );
            verdicts.push(Verdict {
                kind: match o.claim {
                    obligation::Claim::Ensures(_) => ObligationKind::Postcondition,
                    obligation::Claim::Covered => ObligationKind::Coverage,
                    obligation::Claim::Reaches(_) => ObligationKind::RowReachability,
                    obligation::Claim::InRange { .. } => ObligationKind::Overflow,
                    obligation::Claim::Requires { .. } => ObligationKind::Precondition,
                },
                outcome: outcome_of(unit, &o, outcome),
                function: unit.source.name.clone(),
                key: identity::hash(&o.script),
                definition: unit.hash.clone(),
                reused,
                clause: o.clause,
                line: o.line,
            });
        }
    }
    // Each run with whether it is reported: an example in the target's files.
    let runs = program
        .examples
        .iter()
        .map(|example| (example, target.is_none_or(|t| example.file == t)))
        .filter(|(example, reported)| *reported || needed.contains(&example.function))
        .filter_map(|(example, reported)| {
            let function = program
                .functions
                .iter()
                .find(|f| f.name == example.function)?;
            let run = ExampleRun {
                function: function.name.clone(),
                text: example.text.clone(),
                line: example.line,
                run: match interpret::run(example, function, &program.functions) {
                    Ok(()) => Run::Passed,
                    Err(reason) => Run::Failed { reason },
                },
            };
            Some((run, reported))
        })
        .collect::<Vec<_>>();
    let openness = openness::determinacy(&units, solvers);
    let own = |unit: &identity::Unit| {
        standing_of(
            verdicts.iter().filter(|v| v.function == unit.source.name),
            runs.iter()
                .map(|(run, _)| run)
                .filter(|e| e.function == unit.source.name),
            openness
                .iter()
                .find(|d| d.function == unit.source.name)
                .map(|d| &d.openness),
        )
    };
    let own_standings: Vec<(String, Standing)> = units
        .iter()
        .map(|unit| (unit.source.name.clone(), own(unit)))
        .collect();
    let standing_of_name = |name: &str| {
        own_standings
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Standing::Unverified, |(_, s)| *s)
    };
    let definitions: Vec<Definition> = units
        .iter()
        .map(|unit| {
            let unverified_callees: Vec<String> = unit
                .callees
                .iter()
                .filter(|name| standing_of_name(name) != Standing::Verified)
                .cloned()
                .collect();
            let mut standing = standing_of_name(&unit.source.name);
            if standing == Standing::Verified && !unverified_callees.is_empty() {
                standing = Standing::Incomplete;
            }
            Definition {
                name: unit.source.name.clone(),
                intent: unit.source.intent.clone(),
                line: unit.source.line,
                params: unit.source.params.clone(),
                result_type: unit.source.ret.clone(),
                specification: unit
                    .source
                    .clauses
                    .iter()
                    .map(|c| (c.line, c.text.clone()))
                    .collect(),
                enums: unit.source_enums(),
                file: file_of(unit.source.file),
                hash: unit.hash.clone(),
                standing,
                unverified_callees,
            }
        })
        .collect();
    let reported = |name: &str| {
        units
            .iter()
            .any(|unit| unit.source.name == name && targeted(unit))
    };
    let definitions: Vec<Definition> = definitions
        .into_iter()
        .filter(|d| reported(&d.name))
        .collect();
    let verdicts: Vec<Verdict> = verdicts
        .into_iter()
        .filter(|v| reported(&v.function))
        .collect();
    let examples: Vec<ExampleRun> = runs
        .into_iter()
        .filter_map(|(run, reported)| reported.then_some(run))
        .collect();
    let openness: Vec<Determinacy> = openness
        .into_iter()
        .filter(|d| reported(&d.function))
        .collect();
    let units: Vec<_> = units.into_iter().filter(|unit| targeted(unit)).collect();
    let standing = file_standing(&definitions);
    let strength = if solvers.mutates() {
        let proved_units: Vec<_> = units
            .into_iter()
            .filter(|unit| {
                verdicts
                    .iter()
                    .filter(|v| v.function == unit.source.name)
                    .all(|v| matches!(v.outcome, Outcome::Proved { .. }))
            })
            .collect();
        mutation::strengths(&proved_units, solvers)
    } else {
        Vec::new()
    };
    Report {
        verdicts,
        examples,
        definitions,
        standing,
        strength,
        openness,
    }
}
