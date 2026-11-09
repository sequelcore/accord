//! A solver that runs out of time leaves its obligation undecided, and the
//! check still ends on time.

use std::time::{Duration, Instant};

use accord::{Outcome, Solvers, verify};

/// Fermat's last theorem for cubes: true, but neither solver can prove it.
const HARD: &str = "fn no_cubes(a: i64, b: i64, c: i64) -> Bool
    requires a > 0 and b > 0 and c > 0
    ensures a * a * a + b * b * b != c * c * c
{
    true
}
";

#[test]
fn a_solver_timeout_is_undecided_never_proved() {
    let limit = Duration::from_secs(1);
    let solvers = Solvers::from_env()
        .expect("Z3 and cvc5 are available (see conformance/README.md)")
        .with_timeout(limit);
    let start = Instant::now();
    let report = verify(HARD, &solvers).expect("the file checks");
    let elapsed = start.elapsed();
    let [verdict] = report.verdicts.as_slice() else {
        panic!("one obligation expected, got {:?}", report.verdicts);
    };
    let Outcome::Undecided { reason } = &verdict.outcome else {
        panic!("expected undecided, got {:?}", verdict.outcome);
    };
    println!("{reason} after {elapsed:?}");
    assert!(reason.contains("time"), "{reason}");
    let receipt: serde_json::Value =
        serde_json::from_str(&accord::receipt::json(&report, "timeout.accord")).expect("JSON");
    assert_eq!(receipt["standing"], "incomplete");
    assert!(
        receipt["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|d| d["code"] == "ACC0003")
    );
    // Each solver gets the limit once, with time to start and stop.
    assert!(elapsed < limit * 2 + Duration::from_secs(3), "{elapsed:?}");
}
