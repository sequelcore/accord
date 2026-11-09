//! Stage 3's done-when: changing a body makes exactly its own evidence, and
//! its dependents', stale; a rename changes none. Evidence is stale when its
//! obligation is decided again rather than reused from the ledger.

use accord::{Report, Solvers, verify};

const ORIGINAL: &str = "enum Light { Red, Amber, Green }

fn next(light: Light) -> Light
    ensures result != light
decide {
    | light == Red   => Green
    | light == Green => Amber
    | otherwise      => Red
}

fn add(a: i32, b: i32) -> i32
    requires a + b < 100 and a + b > 0
    ensures result == a + b
{
    a + b
}
";

/// Every name differs from `ORIGINAL`, and `add`'s body is `b + a`.
const RENAMED_AND_CHANGED: &str = "enum Signal { Stop, Wait, Go }

fn advance(s: Signal) -> Signal
    ensures result != s
decide {
    | s == Stop => Go
    | s == Go   => Wait
    | otherwise => Stop
}

fn sum(x: i32, y: i32) -> i32
    requires x + y < 100 and x + y > 0
    ensures result == x + y
{
    y + x
}
";

/// For each function, in order, whether each of its obligations was reused.
fn reuse(report: &Report) -> Vec<(String, Vec<bool>)> {
    report
        .definitions
        .iter()
        .map(|d| {
            let reused = report
                .verdicts
                .iter()
                .filter(|v| v.function == d.name)
                .map(|v| v.reused)
                .collect();
            (d.name.clone(), reused)
        })
        .collect()
}

#[test]
fn an_intent_change_updates_the_report_but_reuses_the_proof() {
    let dir = std::env::temp_dir().join(format!("accord-intent-{}", std::process::id()));
    std::fs::create_dir(&dir).expect("new isolated ledger");
    let solvers = Solvers::from_env()
        .expect("Z3 and cvc5 are available (see conformance/README.md)")
        .with_ledger(&dir);
    let original = "fn flag() -> Bool
    intent \"Returns false.\"
    ensures result
{ true }
";
    let first = verify(original, &solvers).expect("first check");
    assert!(all(&first, false));
    assert_eq!(
        first.definitions[0].intent.as_deref(),
        Some("Returns false.")
    );

    let edited = original.replace("Returns false.", "Returns true.");
    let second = verify(&edited, &solvers).expect("intent edit checks");
    assert_eq!(
        second.definitions[0].intent.as_deref(),
        Some("Returns true.")
    );
    assert_eq!(first.definitions[0].hash, second.definitions[0].hash);
    assert_eq!(first.standing, second.standing);
    assert!(all(&second, true), "prose changes no proof obligation");
    let text = accord::receipt::text(&second);
    assert!(text.contains("(unchecked): Returns true."));
    assert!(!text.contains("Returns false."));
    std::fs::remove_dir_all(&dir).expect("remove this test's ledger");
}

fn all(report: &Report, reused: bool) -> bool {
    !report.verdicts.is_empty() && report.verdicts.iter().all(|v| v.reused == reused)
}

#[test]
fn a_change_re_checks_exactly_what_it_touches_and_a_rename_nothing() {
    let dir = std::env::temp_dir().join(format!("accord-ledger-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let solvers = Solvers::from_env()
        .expect("Z3 and cvc5 are available (see conformance/README.md)")
        .with_ledger(&dir);
    let check = |source: &str| verify(source, &solvers).expect("the file checks");

    let first = check(ORIGINAL);
    assert!(all(&first, false), "an empty ledger reuses nothing");
    assert!(
        all(&check(ORIGINAL), true),
        "an unchanged file reuses everything"
    );

    // A body change: only `add`'s evidence is decided again.
    let changed = ORIGINAL.replace("    a + b\n}", "    b + a\n}");
    let after_change = reuse(&check(&changed));
    assert!(after_change[0].1.iter().all(|&r| r), "`next` is reused");
    assert!(after_change[1].1.iter().all(|&r| !r), "`add` is re-checked");

    // Renaming everything in the changed file: nothing is decided again,
    // and results read in the new names.
    let renamed = check(RENAMED_AND_CHANGED);
    assert!(all(&renamed, true), "{:?}", reuse(&renamed));
    assert_eq!(renamed.definitions[0].name, "advance");

    // An enum change reaches only the function that uses it.
    let wider = changed.replace("Green }", "Green, Off }");
    let after_enum = reuse(&check(&wider));
    assert!(after_enum[0].1.iter().any(|&r| !r), "`next` uses `Light`");
    assert!(after_enum[1].1.iter().all(|&r| r), "`add` doesn't");

    // An entry that can't be read is decided again, never trusted.
    let evidence = dir.join("evidence");
    for entry in std::fs::read_dir(&evidence).expect("the ledger exists") {
        std::fs::write(entry.expect("an entry").path(), "not evidence").expect("written");
    }
    assert!(all(&check(ORIGINAL), false), "corrupt entries are misses");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_a_ledger_nothing_is_reused() {
    let solvers = Solvers::from_env().expect("Z3 and cvc5 are available");
    verify(ORIGINAL, &solvers).expect("the file checks");
    assert!(all(&verify(ORIGINAL, &solvers).expect("checks"), false));
}
