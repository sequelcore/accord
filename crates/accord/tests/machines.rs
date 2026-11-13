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

const STEPPED: &str = "machine Pair {
    state { x: i32, y: i32 }
    init start() { next { x: 0, y: 0 } }
    event bump(by: i32)
        requires x < 3 and by >= 1 and by <= 1
    {
        next { x: x + by, y: y + by }
    }
    invariant y_bounded: y <= 3
}
";

#[test]
fn every_input_a_receipt_names_is_typed() {
    let json: serde_json::Value =
        serde_json::from_str(&receipt::json(&report(STEPPED), "pair")).expect("JSON");
    let stuck = json["definitions"][0]["obligations"]
        .as_array()
        .expect("obligations")
        .iter()
        .find(|o| o["outcome"] == "not_inductive")
        .expect("one obligation is not inductive");
    let names: Vec<&str> = stuck["counterexample"]
        .as_array()
        .expect("inputs")
        .iter()
        .map(|input| {
            assert_eq!(input["valid"], true, "{input}");
            input["name"].as_str().expect("a name")
        })
        .collect();
    assert!(
        names.contains(&"by"),
        "the event's parameter is named: {names:?}"
    );
    let allowed = json["definitions"][0]["obligations"]
        .as_array()
        .expect("obligations")
        .iter()
        .find(|o| o["kind"] == "allowed" && o["outcome"] == "proved")
        .expect("the event is allowed");
    assert_eq!(allowed["witness"][0]["name"], "by");
    assert_eq!(allowed["witness"][0]["valid"], true);
}

#[test]
fn a_helper_invariant_is_support_not_a_claim() {
    let source = STEPPED.replace(
        "    invariant y_bounded",
        "    helper invariant together: x == y
    invariant y_bounded",
    );
    let report = report(&source);
    let json: serde_json::Value =
        serde_json::from_str(&receipt::json(&report, "pair")).expect("JSON");
    let machine = &json["definitions"][0];
    assert_eq!(machine["specification"].as_array().map(Vec::len), Some(1));
    assert_eq!(machine["support"].as_array().map(Vec::len), Some(1));
    assert!(
        machine["support"][0]["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("helper invariant"))
    );
    assert!(receipt::text(&report).contains("support      line"));
    assert_eq!(machine["standing"], "verified");
}

#[test]
fn a_machine_s_hash_covers_roles_and_helpers() {
    let hash = |source: &str| report(source).definitions[0].hash.clone();
    let base = "machine M {
    state { x: i32 }
    init a() { next { x: 0 } }
    event b() { next { x: 0 } }
    invariant i: x >= 0
}
";
    let second_init = base.replace("event b()", "init b()");
    let helper = base.replace("    invariant i", "    helper invariant i");
    assert_ne!(hash(base), hash(&second_init));
    assert_ne!(hash(base), hash(&helper));
    assert_eq!(
        hash(base),
        hash(&base.replace("invariant i", "invariant j"))
    );
}
