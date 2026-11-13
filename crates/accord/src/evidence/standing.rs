//! How evidence adds up to a standing, for one definition and for a file.

use crate::{Definition, ExampleRun, Openness, Outcome, Run, Standing, Verdict};

/// A definition's standing from its own evidence.
pub(super) fn standing_of<'a>(
    verdicts: impl Iterator<Item = &'a Verdict>,
    examples: impl Iterator<Item = &'a ExampleRun>,
    openness: Option<&Openness>,
) -> Standing {
    let (mut any, mut failed, mut open) = (false, false, false);
    // An open case, or a search for one that didn't settle, keeps a
    // definition from verified; whether a freedom is intended is a person's
    // decision, so it doesn't fail it.
    open |= matches!(
        openness,
        Some(Openness::Open { .. } | Openness::Undecided { .. })
    );
    for verdict in verdicts {
        any = true;
        match verdict.outcome {
            Outcome::Proved { .. } => {}
            Outcome::Refuted { .. } => failed = true,
            Outcome::Undecided { .. }
            | Outcome::Unsupported { .. }
            | Outcome::WeakCallee { .. }
            | Outcome::NotInductive { .. } => open = true,
        }
    }
    for example in examples {
        any = true;
        failed |= example.run != Run::Passed;
    }
    match (failed, open, any) {
        (true, _, _) => Standing::Failed,
        (false, true, _) => Standing::Incomplete,
        (false, false, true) => Standing::Verified,
        (false, false, false) => Standing::Unverified,
    }
}

/// A file's standing: failed if any definition failed, verified only if
/// every one is, unverified if none has evidence, and incomplete otherwise.
pub(super) fn file_standing(definitions: &[Definition]) -> Standing {
    let all = |s: Standing| definitions.iter().all(|d| d.standing == s);
    if definitions.iter().any(|d| d.standing == Standing::Failed) {
        Standing::Failed
    } else if !definitions.is_empty() && all(Standing::Verified) {
        Standing::Verified
    } else if all(Standing::Unverified) {
        Standing::Unverified
    } else {
        Standing::Incomplete
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Definition, ExampleRun, Outcome, Run, Standing, Verdict, file_standing, standing_of,
    };

    fn verdict(outcome: Outcome) -> Verdict {
        Verdict {
            kind: crate::ObligationKind::Postcondition,
            function: "f".to_string(),
            clause: "ensures result".to_string(),
            line: 1,
            outcome,
            key: String::new(),
            definition: String::new(),
            reused: false,
            exploration: None,
            parameters: Vec::new(),
        }
    }

    fn example(run: Run) -> ExampleRun {
        ExampleRun {
            function: "f".to_string(),
            text: "example f() == true".to_string(),
            line: 2,
            run,
        }
    }

    #[test]
    fn a_definition_stands_on_its_own_evidence() {
        let proved = verdict(Outcome::Proved { witness: None });
        let refuted = verdict(Outcome::Refuted {
            evidence: crate::Refutation::NoSuchInput,
        });
        let undecided = verdict(Outcome::Undecided {
            reason: "timeout".to_string(),
        });
        let failed = example(Run::Failed {
            reason: "no".to_string(),
        });
        let of = |v: &[&Verdict], e: &[&ExampleRun]| {
            standing_of(v.iter().copied(), e.iter().copied(), None)
        };
        assert_eq!(of(&[&proved], &[&example(Run::Passed)]), Standing::Verified);
        assert_eq!(of(&[&proved, &undecided], &[]), Standing::Incomplete);
        assert_eq!(of(&[&undecided, &refuted], &[]), Standing::Failed);
        assert_eq!(of(&[&proved], &[&failed]), Standing::Failed);
        assert_eq!(of(&[], &[]), Standing::Unverified);
        let open = crate::Openness::Open {
            inputs: Vec::new(),
            results: ("true".to_string(), "false".to_string()),
            code: "true".to_string(),
        };
        let proved_only = [&proved];
        assert_eq!(
            standing_of(proved_only.iter().copied(), [].iter(), Some(&open)),
            Standing::Incomplete,
            "an open case keeps a proved definition from verified"
        );
        let failing = [&refuted];
        assert_eq!(
            standing_of(failing.iter().copied(), [].iter(), Some(&open)),
            Standing::Failed
        );
    }

    #[test]
    fn a_file_is_verified_only_when_every_definition_is() {
        let with = |standings: &[Standing]| {
            let definitions: Vec<Definition> = standings
                .iter()
                .map(|&standing| Definition {
                    kind: crate::DefinitionKind::Function,
                    support: Vec::new(),
                    intent: None,
                    line: 1,
                    params: Vec::new(),
                    result_type: Some(crate::Type::Bool),
                    specification: Vec::new(),
                    enums: Vec::new(),
                    name: "f".to_string(),
                    file: None,
                    hash: String::new(),
                    standing,
                    unverified_callees: Vec::new(),
                })
                .collect();
            file_standing(&definitions)
        };
        use Standing::{Failed, Incomplete, Unverified, Verified};
        assert_eq!(with(&[Verified, Verified]), Verified);
        assert_eq!(with(&[Verified, Unverified]), Incomplete);
        assert_eq!(with(&[Verified, Incomplete]), Incomplete);
        assert_eq!(with(&[Incomplete, Failed]), Failed);
        assert_eq!(with(&[Unverified]), Unverified);
        assert_eq!(with(&[]), Unverified);
    }
}
