//! What the conformance cases leave open about machines: how events with
//! parameters and calls are explored, what an exploration that cannot finish
//! says, and what the receipt and `build` do with a machine.

use accord::{Outcome, Refutation, Report, Solvers, build, receipt, verify};

const DIAL: &str = "enum Mode { Idle, Busy }

machine Dial {
    state {
        level: i32,
        mode: Mode,
    }

    init start(initial: i32)
        requires initial >= 0 and initial <= 2
    {
        next { level: initial, mode: Idle }
    }

    event set(to: i32, flag: Bool)
        requires to >= 0 and to <= 12
    {
        if flag {
            next { level: to, mode: Busy }
        } else {
            next { level: to }
        }
    }

    invariant bounded: level <= 10
}
";

fn report(source: &str) -> Report {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    verify(source, &solvers).expect("the machine is accepted")
}

fn clauses_of(report: &Report) -> Vec<(&str, &Outcome)> {
    report
        .verdicts
        .iter()
        .map(|v| (v.clause.as_str(), &v.outcome))
        .collect()
}

#[test]
fn an_event_with_parameters_is_refuted_by_the_shortest_run() {
    let report = report(DIAL);
    let preserves = report
        .verdicts
        .iter()
        .find(|v| v.clause.contains("`set` preserves"))
        .expect("the event preserves its invariants");
    let Outcome::Refuted {
        evidence: Refutation::Trace(steps),
    } = &preserves.outcome
    else {
        panic!("not refuted by a run: {:?}", preserves.outcome);
    };
    let calls: Vec<&str> = steps.iter().map(|s| s.call.as_str()).collect();
    assert_eq!(calls, ["start(0)", "set(to: 11, flag: false)"]);
    let exploration = preserves.exploration.as_ref().expect("annotated");
    assert!(exploration.complete);
    assert_eq!(exploration.states, 26);
}

#[test]
fn a_parameter_without_known_values_leaves_exploration_incomplete() {
    let source = DIAL.replace("requires to >= 0 and to <= 12", "requires to >= 0");
    let report = report(&source);
    let allowed = report
        .verdicts
        .iter()
        .find(|v| v.clause.contains("`set` is allowed"))
        .expect("the event is allowed somewhere");
    assert!(
        matches!(allowed.outcome, Outcome::Undecided { .. }),
        "the solvers' state is not a run: {:?}",
        clauses_of(&report)
    );
    let exploration = allowed.exploration.as_ref().expect("annotated");
    assert!(!exploration.complete);
}

#[test]
fn the_receipt_names_the_outcome_and_the_exploration() {
    let source = "machine Pair {
    state { x: i32, y: i32 }
    init start() { next { x: 0, y: 0 } }
    event step() requires x < 3 { next { x: x + 1, y: y + 1 } }
    invariant y_bounded: y <= 3
}
";
    let report = report(source);
    let json: serde_json::Value =
        serde_json::from_str(&receipt::json(&report, "pair")).expect("JSON");
    assert_eq!(json["receipt"], 4);
    let definition = &json["definitions"][0];
    assert_eq!(definition["kind"], "machine");
    let stuck = definition["obligations"]
        .as_array()
        .expect("obligations")
        .iter()
        .find(|o| o["outcome"] == "not_inductive")
        .expect("one obligation is not inductive");
    assert_eq!(stuck["kind"], "invariant");
    assert_eq!(stuck["exploration"]["complete"], true);
    assert_eq!(json["diagnostics"][0]["code"], "ACC0015");
    assert_eq!(json["diagnostics"][0]["action"], "add_helper_invariant");
    assert_eq!(json["standing"], "incomplete");
}

#[test]
fn a_machine_is_not_written_as_typescript() {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    let files = [("dial.accord".to_string(), DIAL.to_string())];
    let Err(refusal) = build(&files, &solvers) else {
        panic!("a machine was built");
    };
    assert!(refusal.message.contains("machine"), "{}", refusal.message);
}
