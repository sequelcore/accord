//! Differential testing: every program in `conformance` that builds is
//! written as TypeScript, run in Node with type stripping, and compared with
//! the interpreter on the same inputs. Agreement on these inputs is evidence
//! the code generator is right, not a proof.
//!
//! The inputs for each exported function are every combination of its
//! parameters' values: both `Bool`s, every variant, each width's boundaries
//! and a seeded sample for integers, and for texts the empty text, every text
//! literal in the program and characters beyond ASCII. Each `example`'s call
//! and every input a solver gave are added. Past 4,096 combinations, a seeded
//! sample of them is taken.
//!
//! An input the boundary must refuse is expected to throw: a broken
//! `requires` or an `i64` that isn't a safe integer throws a `RangeError`.
//! Node must be on the `PATH`, or named by `ACCORD_NODE`; without it the
//! test fails rather than skipping.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value as Json, json};

mod inputs;

use super::expr::camel;
use crate::Solvers;
use crate::interpret::{self, Value};
use crate::syntax::{Enum, Function, Type, Width};
use inputs::{SAFE, Seeded, inputs};

fn to_json(value: &Value, enums: &[Enum]) -> Json {
    match value {
        Value::Bool(b) => json!(b),
        Value::Int(n) => i64::try_from(*n).map_or_else(|_| json!(n.to_string()), |n| json!(n)),
        Value::Text(t) => json!(t),
        Value::List(items) => Json::Array(items.iter().map(|v| to_json(v, enums)).collect()),
        Value::Variant(owner, name) => {
            let e = enums.iter().find(|e| e.name == *owner);
            json!(e.expect("a variant of a declared enum").text(name))
        }
    }
}

/// What the generated function must do for `names`.
fn expected(
    function: &Function,
    names: &interpret::Names,
    functions: &[Function],
    enums: &[Enum],
) -> Json {
    let unsafe_i64 = |ty: &Type, v: &Value| {
        *ty == Type::Int(Width::I64) && matches!(v, Value::Int(n) if n.abs() > SAFE)
    };
    if function
        .params
        .iter()
        .zip(names)
        .any(|((_, ty), (_, v))| unsafe_i64(ty, v))
    {
        return json!({"error": "RangeError"});
    }
    match interpret::broken_requires(function, names) {
        Ok(Some(_)) => return json!({"error": "RangeError"}),
        Ok(None) => {}
        Err(reason) => panic!("{}: requires can't be evaluated: {reason}", function.name),
    }
    match interpret::result(function, names, functions) {
        Ok(result) if unsafe_i64(&function.ret, &result) => json!({"error": "RangeError"}),
        Ok(result) => json!({"ok": to_json(&result, enums)}),
        Err(reason) => panic!(
            "{}: no result for an admitted input: {reason}",
            function.name
        ),
    }
}

fn conformance() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

/// Every program in `conformance`: each case file alone, each folder of
/// `programs` and each `src` folder of `typescript`.
fn programs() -> Vec<(String, Vec<(String, String)>)> {
    let read = |path: &Path| std::fs::read_to_string(path).expect("readable");
    // A checked folder holds a ledger in `.accord`, which is no source.
    let entries = |dir: PathBuf, folders: bool| -> Vec<PathBuf> {
        let mut all: Vec<PathBuf> = std::fs::read_dir(dir)
            .expect("a conformance folder")
            .map(|e| e.expect("an entry").path())
            .filter(|p| {
                if folders {
                    p.is_dir() && !p.ends_with(".accord")
                } else {
                    p.is_file() && p.extension().is_some_and(|e| e == "accord")
                }
            })
            .collect();
        all.sort();
        all
    };
    let folder = |dir: &Path| -> Vec<(String, String)> {
        entries(dir.to_path_buf(), false)
            .into_iter()
            .map(|p| {
                (
                    p.file_name()
                        .expect("a name")
                        .to_string_lossy()
                        .into_owned(),
                    read(&p),
                )
            })
            .collect()
    };
    let mut found = Vec::new();
    for path in entries(conformance().join("cases"), false) {
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        found.push((name.clone(), vec![(name, read(&path))]));
    }
    for dir in entries(conformance().join("programs"), true) {
        found.push((dir.display().to_string(), folder(&dir)));
    }
    for dir in entries(conformance().join("typescript"), true) {
        found.push((dir.display().to_string(), folder(&dir.join("src"))));
    }
    found
}

#[test]
fn generated_modules_agree_with_the_interpreter() {
    let solvers =
        Solvers::from_env().expect("Z3 and cvc5 are available (see conformance/README.md)");
    let root = std::env::temp_dir().join(format!("accord-differential-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a temporary folder");
    std::fs::write(root.join("package.json"), "{\"type\":\"module\"}\n").expect("written");
    let mut seeded = Seeded(3);
    let (mut calls, mut wanted, mut built) = (Vec::new(), Vec::new(), 0);
    for (index, (name, files)) in programs().into_iter().enumerate() {
        let Ok((program, names)) = crate::program_of(&files) else {
            continue;
        };
        let report = crate::report(&program, None, &names, &solvers);
        if !super::proved(&report) {
            continue;
        }
        built += 1;
        let dir = root.join(index.to_string());
        std::fs::create_dir_all(&dir).expect("a module folder");
        for (module, source) in super::modules(&program, &names, &report.openness) {
            std::fs::write(dir.join(module), source).expect("written");
        }
        for function in program.functions.iter().filter(|f| f.public) {
            let module = dir.join(format!(
                "{}.ts",
                names[function.file]
                    .strip_suffix(".accord")
                    .unwrap_or(&names[function.file])
            ));
            for args in inputs(&program, &report, function, &mut seeded) {
                let names: Vec<(String, Value)> = function
                    .params
                    .iter()
                    .map(|(n, _)| n.clone())
                    .zip(args.iter().cloned())
                    .collect();
                wanted.push((
                    name.clone(),
                    function.name.clone(),
                    expected(function, &names, &program.functions, &program.enums),
                ));
                calls.push(json!({
                    "module": module.display().to_string(),
                    "name": camel(&function.name),
                    "args": args.iter().map(|a| to_json(a, &program.enums)).collect::<Vec<_>>(),
                }));
            }
        }
    }
    assert!(built > 0, "no program built");
    let runner = root.join("runner.mjs");
    std::fs::write(&runner, RUNNER).expect("written");
    let calls_file = root.join("calls.json");
    std::fs::write(&calls_file, Json::Array(calls.clone()).to_string()).expect("written");
    let node = std::env::var("ACCORD_NODE").unwrap_or_else(|_| "node".to_string());
    let output = Command::new(&node)
        .arg("--experimental-strip-types")
        .arg(&runner)
        .arg(&calls_file)
        .output()
        .unwrap_or_else(|error| panic!("`{node}` runs (Node 22.6 or later): {error}"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let got: Vec<Json> = serde_json::from_slice(&output.stdout).expect("the runner's JSON");
    assert_eq!(got.len(), wanted.len());
    let differing: Vec<String> = wanted
        .iter()
        .zip(&got)
        .zip(&calls)
        .filter(|(((.., want), got), _)| want != *got)
        .map(|(((program, function, want), got), call)| {
            format!(
                "{program}: {function}({}): want {want}, got {got}",
                call["args"]
            )
        })
        .take(20)
        .collect();
    assert!(
        differing.is_empty(),
        "{} programs, {} calls:\n{}",
        built,
        wanted.len(),
        differing.join("\n")
    );
    let _ = std::fs::remove_dir_all(&root);
}

const RUNNER: &str = r#"import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const calls = JSON.parse(readFileSync(process.argv[2], "utf8"));
const results = [];
for (const call of calls) {
  const module = await import(pathToFileURL(call.module).href);
  try {
    results.push({ ok: module[call.name](...call.args) });
  } catch (error) {
    results.push({ error: error.constructor.name });
  }
}
process.stdout.write(JSON.stringify(results));
"#;
