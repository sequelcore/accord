//! Two solvers' answers become one outcome, and an outcome that rests on a
//! model stands only if the interpreter reproduces it.

use super::reply::Answer;
use crate::interpret::Replay;
use crate::obligation::Polarity;
use crate::{Inputs, Outcome, Refutation};

/// Two answers become one outcome. Only agreement proves or refutes. A
/// universal obligation's script asserts that it fails, so `unsat` proves it;
/// an existential one's asserts the input it needs, so `sat` proves it.
pub(crate) fn combine(z3: &Answer, cvc5: &Answer, polarity: Polarity) -> Outcome {
    match (z3, cvc5, polarity) {
        (Answer::Unsat, Answer::Unsat, Polarity::Universal) => Outcome::Proved { witness: None },
        (Answer::Unsat, Answer::Unsat, Polarity::Existential) => Outcome::Refuted {
            evidence: Refutation::NoSuchInput,
        },
        (Answer::Sat(values), Answer::Sat(_), Polarity::Universal) => Outcome::Refuted {
            evidence: Refutation::Counterexample(values.clone()),
        },
        (Answer::Sat(values), Answer::Sat(_), Polarity::Existential) => Outcome::Proved {
            witness: Some(values.clone()),
        },
        (Answer::Sat(_), Answer::Unsat, _) | (Answer::Unsat, Answer::Sat(_), _) => {
            Outcome::Undecided {
                reason: "the solvers disagree: one found an input and the other proved none exists"
                    .to_string(),
            }
        }
        (Answer::Unknown(reason), _, _) => Outcome::Undecided {
            reason: format!("Z3: {reason}"),
        },
        (_, Answer::Unknown(reason), _) => Outcome::Undecided {
            reason: format!("cvc5: {reason}"),
        },
    }
}

/// An outcome that rests on models stands only if `replay` reproduces each
/// solver's model; otherwise it is undecided. A counterexample that
/// reproduces only with each call taking the model's value rests on a
/// callee's contract, not on the code: unless the other solver's reproduces
/// as the code runs, the outcome is that a callee's contract is too weak.
pub(crate) fn confirm(
    outcome: Outcome,
    z3: &Answer,
    cvc5: &Answer,
    replay: impl Fn(&Inputs) -> Result<Replay, String>,
) -> Outcome {
    let kind = match &outcome {
        Outcome::Proved { witness: Some(_) } => "witness",
        Outcome::Refuted {
            evidence: Refutation::Counterexample(_),
        } => "counterexample",
        _ => return outcome,
    };
    let (mut runs, mut assumed) = (None, None);
    for (solver, answer) in [("Z3", z3), ("cvc5", cvc5)] {
        let Answer::Sat(model) = answer else {
            continue;
        };
        match replay(model) {
            Ok(Replay::Runs) => {
                runs.get_or_insert_with(|| model.clone());
            }
            Ok(Replay::Assumed) if kind == "counterexample" => {
                assumed.get_or_insert_with(|| model.clone());
            }
            Ok(Replay::Assumed) => {
                return Outcome::Undecided {
                    reason: format!(
                        "{solver}'s witness reaches it only with a call result its callee's contract allows and its body doesn't give"
                    ),
                };
            }
            Err(why) => {
                return Outcome::Undecided {
                    reason: format!("{solver}'s {kind} doesn't reproduce: {why}"),
                };
            }
        }
    }
    match (outcome, runs, assumed) {
        (
            Outcome::Refuted {
                evidence: Refutation::Counterexample(_),
            },
            Some(model),
            _,
        ) => Outcome::Refuted {
            evidence: Refutation::Counterexample(model),
        },
        (_, None, Some(model)) => Outcome::WeakCallee {
            inputs: model,
            calls: Vec::new(),
        },
        (outcome, ..) => outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, Replay, combine, confirm};
    use crate::obligation::Polarity::{Existential, Universal};
    use crate::{Outcome, Refutation};

    #[test]
    fn only_two_unsat_answers_prove_a_universal_obligation() {
        assert_eq!(
            combine(&Answer::Unsat, &Answer::Unsat, Universal),
            Outcome::Proved { witness: None }
        );
        let unknown = Answer::Unknown("ran out of time".to_string());
        for polarity in [Universal, Existential] {
            assert!(matches!(
                combine(&Answer::Unsat, &unknown, polarity),
                Outcome::Undecided { .. }
            ));
            assert!(matches!(
                combine(&unknown, &Answer::Unsat, polarity),
                Outcome::Undecided { .. }
            ));
        }
    }

    #[test]
    fn an_existential_obligation_is_proved_by_a_witness_and_refuted_by_two_unsat() {
        let sat = Answer::Sat(vec![("a".to_string(), "true".to_string())]);
        assert_eq!(
            combine(&sat, &sat, Existential),
            Outcome::Proved {
                witness: Some(vec![("a".to_string(), "true".to_string())])
            }
        );
        assert_eq!(
            combine(&Answer::Unsat, &Answer::Unsat, Existential),
            Outcome::Refuted {
                evidence: Refutation::NoSuchInput
            }
        );
        let unknown = Answer::Unknown("ran out of time".to_string());
        assert!(matches!(
            combine(&sat, &unknown, Existential),
            Outcome::Undecided { .. }
        ));
    }

    #[test]
    fn disagreement_is_undecided() {
        let sat = Answer::Sat(vec![("a".to_string(), "true".to_string())]);
        for polarity in [Universal, Existential] {
            assert!(matches!(
                combine(&sat, &Answer::Unsat, polarity),
                Outcome::Undecided { .. }
            ));
            assert!(matches!(
                combine(&Answer::Unsat, &sat, polarity),
                Outcome::Undecided { .. }
            ));
        }
    }

    #[test]
    fn a_model_that_does_not_replay_leaves_the_obligation_undecided() {
        let a = |v: &str| Answer::Sat(vec![("a".to_string(), v.to_string())]);
        let (good, bad) = (a("1"), a("2"));
        let replay = |model: &crate::Inputs| match model[0].1.as_str() {
            "1" => Ok(Replay::Runs),
            _ => Err("no".to_string()),
        };
        for polarity in [Universal, Existential] {
            let agreed = combine(&good, &good, polarity);
            assert_eq!(confirm(agreed.clone(), &good, &good, replay), agreed);
            let undecided = |z3: &Answer, cvc5: &Answer| {
                matches!(
                    confirm(combine(z3, cvc5, polarity), z3, cvc5, replay),
                    Outcome::Undecided { .. }
                )
            };
            assert!(undecided(&bad, &good), "Z3's model is checked");
            assert!(undecided(&good, &bad), "cvc5's model is checked too");
        }
        // An outcome that rests on `unsat` has no model to replay.
        let proved = combine(&Answer::Unsat, &Answer::Unsat, Universal);
        assert_eq!(
            confirm(proved.clone(), &Answer::Unsat, &Answer::Unsat, |_| Err(
                "never called".to_string()
            )),
            proved
        );
    }
}
