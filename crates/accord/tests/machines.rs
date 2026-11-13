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
fn only_a_proved_pub_machine_is_written_as_typescript() {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    let build_of =
        |source: &str| build(&[("dial.accord".to_string(), source.to_string())], &solvers);
    // `bounded` is refuted, so nothing is written.
    let built = build_of(DIAL).expect("the program is accepted");
    assert!(built.modules.is_none());
    let proved = DIAL.replace("level <= 10", "level <= 12");
    let built = build_of(&proved.replace("machine Dial", "pub machine Dial"))
        .expect("the program is accepted");
    let modules = built.modules.expect("every obligation is proved");
    assert!(
        modules[0]
            .source
            .contains("export function set(state$: Dial")
    );
    // A machine without `pub` writes nothing.
    let built = build_of(&proved).expect("the program is accepted");
    let modules = built.modules.expect("every obligation is proved");
    assert!(!modules[0].source.contains("Dial"));
}

#[test]
fn names_that_would_collide_in_a_module_are_refused() {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    let source = "pub fn start() -> i32 { 1 }
pub machine M {
    state { x: i32 }
    init start() { next { x: 0 } }
}
";
    let Err(refusal) = build(&[("m.accord".to_string(), source.to_string())], &solvers) else {
        panic!("colliding names were built");
    };
    assert!(refusal.message.contains("start"), "{}", refusal.message);
}

#[test]
fn an_enum_and_a_machine_may_not_share_an_exported_type_name() {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    let source = "pub enum M { A }
pub machine M {
    state { x: i32 }
    init start() { next { x: 0 } }
}
";
    let Err(refusal) = build(&[("m.accord".to_string(), source.to_string())], &solvers) else {
        panic!("colliding type names were built");
    };
    assert!(refusal.message.contains("type M"), "{}", refusal.message);
}

/// Writes the modules `source` builds to a folder of its own and runs
/// `script`, a module that imports `./m.ts`, in Node; returns its output.
fn node(source: &str, script: &str) -> String {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    let built = build(&[("m.accord".to_string(), source.to_string())], &solvers)
        .expect("the program is accepted");
    let modules = built.modules.expect("every obligation is proved");
    let dir = std::env::temp_dir().join(format!(
        "accord-machines-{}-{}",
        std::process::id(),
        script.len()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    std::fs::write(dir.join("package.json"), "{\"type\":\"module\"}\n").expect("written");
    for module in &modules {
        std::fs::write(dir.join(&module.name), &module.source).expect("written");
    }
    std::fs::write(dir.join("run.mjs"), script).expect("written");
    let node = std::env::var("ACCORD_NODE").unwrap_or_else(|_| "node".to_string());
    let output = std::process::Command::new(node)
        .arg("--experimental-strip-types")
        .arg(dir.join("run.mjs"))
        .output()
        .expect("Node runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8(output.stdout).expect("text")
}

#[test]
fn a_state_is_read_once_and_the_event_runs_on_what_was_checked() {
    let source = "pub machine Counter {
    state { presses: i32 }
    init start() { next { presses: 0 } }
    event press() requires presses < 100 { next { presses: presses + 1 } }
    event keep() { next { } }
    fn doubled() -> i32 { presses }
    invariant pressed: presses >= 0 and presses <= 100
}
";
    let out = node(
        source,
        "import * as m from './m.ts';
const attack = () => { let reads = 0; return { get presses() { return reads++ === 0 ? 5 : -5; } }; };
console.log(JSON.stringify([
  m.press(attack()).presses,
  m.keep(attack()).presses,
  m.doubled(attack()),
]));
",
    );
    assert_eq!(out.trim(), "[6,5,5]");
}

#[test]
fn fields_named_state_and_where_build_and_import() {
    let source = "pub machine Odd {
    state { state: i32, where: i32 }
    init start() { next { state: 0, where: 1 } }
    event swap() { next { state: where, where: state } }
    invariant small: state >= 0 and state <= 1 and where >= 0 and where <= 1
}
";
    let out = node(
        source,
        "import * as m from './m.ts';
const s = m.swap(m.start());
console.log(JSON.stringify(s));
",
    );
    assert_eq!(out.trim(), "{\"state\":1,\"where\":0}");
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
