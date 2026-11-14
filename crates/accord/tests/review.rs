//! The review view must describe the contract without requiring the body.
use accord::{Solvers, receipt, verify};

#[test]
fn review_contains_the_domain_contract_evidence_and_limits_without_the_body() {
    let source = "enum Decision { Deny, Allow }
fn allowed(n: i64) -> Decision
    intent \"Allow positive counts.\"
    requires n >= 0
    ensures result == Allow implies n > 0
    ensures n > 0 implies result == Allow
{ if n > 0 { Allow } else { Deny } }
example allowed(0) == Deny
";
    let solvers = Solvers::from_env().expect("solvers").with_mutation();
    let report = verify(source, &solvers).expect("checked");
    let json: serde_json::Value =
        serde_json::from_str(&receipt::json(&report, "review.accord")).expect("JSON");
    let d = &json["definitions"][0];
    assert_eq!(d["signature"], "fn allowed(n: i64) -> Decision");
    assert_eq!(d["specification"][0]["text"], "requires n >= 0");
    assert_eq!(d["specification"].as_array().unwrap().len(), 3);
    assert_eq!(
        d["types"][0]["variants"],
        serde_json::json!(["Deny", "Allow"])
    );
    assert_eq!(d["review_hash"].as_str().map(str::len), Some(64));
    assert!(d.get("body").is_none());
    assert!(
        d["not_proved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["kind"] == "intent_alignment")
    );
    let text = receipt::text(&report);
    for expected in [
        "fn allowed(n: i64) -> Decision",
        "enum Decision { Deny, Allow }",
        "requires n >= 0",
        "example allowed(0) == Deny",
        "not_proved",
        "intent_alignment",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(!text.contains("if n > 0 { Allow } else { Deny }"));
}

#[test]
fn row_reachability_is_existential_evidence_without_printing_the_body() {
    let solvers = Solvers::from_env().expect("solvers");
    let source = "fn identity(a: Bool) -> Bool\n    ensures result == a\ndecide {\n | a => true\n | otherwise => false\n}\n";
    let report = verify(source, &solvers).expect("checked");
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
    let row = &j["definitions"][0]["obligations"][2];
    assert_eq!(row["kind"], "row_reachability");
    assert_eq!(row["polarity"], "existential");
    assert_eq!(row["witness"][0]["value"], true);
    assert!(!receipt::text(&report).contains("| a => true"));
}

#[test]
fn review_identity_changes_with_intent_but_not_cache_reuse() {
    let solvers = Solvers::from_env().unwrap();
    let source = "fn flag() -> Bool\n intent \"Original purpose.\"\n ensures result\n{ true }\nexample flag() == true";
    let first = verify(source, &solvers).unwrap();
    let read = |report: &accord::Report| -> serde_json::Value {
        serde_json::from_str(&receipt::json(report, "rule")).unwrap()
    };
    let a = read(&first);
    let mut reused = first.clone();
    for v in &mut reused.verdicts {
        v.reused = true;
    }
    assert_eq!(
        a["definitions"][0]["review_hash"],
        read(&reused)["definitions"][0]["review_hash"]
    );
    let edited = verify(
        &source.replace("Original purpose.", "Changed purpose."),
        &solvers,
    )
    .unwrap();
    let b = read(&edited);
    assert_eq!(a["definitions"][0]["hash"], b["definitions"][0]["hash"]);
    assert_ne!(
        a["definitions"][0]["review_hash"],
        b["definitions"][0]["review_hash"]
    );
    let examples = verify(
        &source.replace("example flag() == true", "example flag() == false"),
        &solvers,
    )
    .unwrap();
    assert_ne!(
        a["definitions"][0]["review_hash"],
        read(&examples)["definitions"][0]["review_hash"]
    );
}

#[test]
fn contract_strength_is_not_claimed_for_a_refuted_baseline() {
    let solvers = Solvers::from_env().unwrap().with_mutation();
    let source = "fn wrong(a: Bool) -> Bool\n ensures result == a\n{ not a }";
    let report = verify(source, &solvers).unwrap();
    assert_eq!(report.standing, accord::Standing::Failed);
    assert!(
        report.strength.is_empty(),
        "fixing a bad body is not evidence of a weak contract"
    );
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
    assert!(j["definitions"][0]["strength"].is_null());
    assert!(receipt::text(&report).contains("mutation_not_run"));
}

#[test]
fn enum_list_counterexamples_use_source_variants_in_typed_values() {
    let solvers = Solvers::from_env().unwrap();
    let source = "enum Verdict { Accept, Retry }\nfn wrong(items: List<Verdict>) -> Bool\n ensures not result\n{ Verdict.Retry in items }";
    let report = verify(source, &solvers).unwrap();
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
    let input = &j["definitions"][0]["obligations"][0]["counterexample"][0];
    assert_eq!(input["valid"], true, "{input}");
    assert!(
        input["value"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("Retry"))
    );
    assert!(input["source"].as_str().unwrap().contains("Verdict.Retry"));
}

#[test]
fn record_and_variant_counterexamples_decode_their_fields() {
    let solvers = Solvers::from_env().unwrap();
    let source = "enum Step { Stay, Add(amount: i32) }
record Pair {
    left: Step,
    total: i64,
}
fn wrong(pair: Pair) -> Bool
    ensures not result
{ pair.left != Stay and pair.total > 0 }";
    let report = verify(source, &solvers).unwrap();
    let j: serde_json::Value = serde_json::from_str(&receipt::json(&report, "rule")).unwrap();
    let input = &j["definitions"][0]["obligations"][0]["counterexample"][0];
    assert_eq!(input["valid"], true, "{input}");
    assert_eq!(
        input["type"],
        serde_json::json!({"kind": "record", "name": "Pair"})
    );
    assert_eq!(input["value"]["left"]["variant"], "Add", "{input}");
    assert!(input["value"]["left"]["fields"]["amount"].is_string());
    assert!(input["value"]["total"].is_string());
    let written = input["source"].as_str().unwrap();
    assert!(
        written.starts_with("Pair { left: Step.Add(amount: "),
        "{written}"
    );
}
