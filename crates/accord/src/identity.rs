//! A definition's identity is its meaning, not its names. Each function is
//! normalized before any obligation is generated: parameters become `p0`,
//! `p1`, ..., the functions it calls `f0`, `f1`, ... in order of first call,
//! the enums it uses become `E0`, `E1`, ... in order of first use, and their
//! variants `v0`, `v1`, ... by position. Obligations come from the normalized
//! function, so a rename changes no script, and a script declares only the
//! enums its function and its callees' contracts use.
//!
//! A caller is proved through its callees' contracts, never their bodies, so
//! its identity covers each callee's contract and not its body: changing only
//! a callee's body leaves its callers' evidence current.
//!
//! What the solvers say about a normalized function is translated back to
//! source names only for people to read.

mod normalize;
mod source;

use normalize::{Renamer, canonical};
use sha2::{Digest, Sha256};

use crate::syntax::{Enum, Function, Program};

/// One function, normalized, with what's needed to read results back.
pub(crate) struct Unit<'a> {
    /// The normalized function first, then every function it calls, directly
    /// or not, as `f0`, `f1`, ...; its direct callees come first among them.
    /// Its enums are those the function and its direct callees' contracts
    /// use, the only ones a script declares.
    pub program: Program,
    /// The function as written.
    pub source: &'a Function,
    /// Each normalized enum's source declaration, by index, including those
    /// only a callee's body uses.
    enums: Vec<&'a Enum>,
    /// The source name of each normalized callee, by index.
    pub callees: Vec<String>,
    /// The hash of every callee's body, directly called or not, which a
    /// replayed model depends on; empty when it calls nothing.
    pub callee_bodies: String,
    /// The hash of the normalized function and its direct callees'
    /// contracts: its definition's identity.
    pub hash: String,
}

/// SHA-256 of `text`, in hex.
pub(crate) fn hash(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

/// SHA-256 of `bytes`, in hex.
pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every function of a checked program, normalized, in source order.
pub(crate) fn units(program: &Program) -> Vec<Unit<'_>> {
    program
        .functions
        .iter()
        .map(|source| {
            let mut renamer = Renamer {
                program,
                enums: Vec::new(),
                functions: Vec::new(),
            };
            let mut function = renamer.contract(source);
            renamer.body(&mut function, &source.params);
            // The direct callees' contracts, then every callee's body: the
            // enums found before the bodies are the ones a script declares.
            let direct = renamer.functions.len();
            let mut callees: Vec<Function> = (0..direct)
                .map(|index| {
                    let callee = renamer.functions[index];
                    let mut normalized = renamer.contract(callee);
                    normalized.name = format!("f{index}");
                    normalized
                })
                .collect();
            let visible = renamer.enums.len();
            let mut index = 0;
            while index < renamer.functions.len() {
                let callee = renamer.functions[index];
                if index >= callees.len() {
                    let mut normalized = renamer.contract(callee);
                    normalized.name = format!("f{index}");
                    callees.push(normalized);
                }
                renamer.body(&mut callees[index], &callee.params);
                index += 1;
            }
            let enums: Vec<Enum> = renamer.enums[..visible]
                .iter()
                .enumerate()
                .map(|(index, e)| Enum {
                    name: format!("E{index}"),
                    variants: (0..e.variants.len()).map(|v| format!("v{v}")).collect(),
                    values: vec![None; e.variants.len()],
                    line: 0,
                    file: 0,
                })
                .collect();
            let mut text = String::from("(fn");
            for e in &enums {
                text += &format!(" (enum {} {})", e.name, e.variants.len());
            }
            text += &canonical(&function, true);
            for callee in &callees[..direct] {
                text += &format!(" (callee {}{})", callee.name, canonical(callee, false));
            }
            let hash = hash(&(text + ")"));
            let callee_names = renamer.functions.iter().map(|f| f.name.clone()).collect();
            let callee_bodies = if callees.is_empty() {
                String::new()
            } else {
                self::hash(
                    &callees
                        .iter()
                        .map(|c| format!("(callee {}{})", c.name, canonical(c, true)))
                        .collect::<String>(),
                )
            };
            let mut functions = vec![function];
            functions.extend(callees);
            Unit {
                program: Program {
                    enums,
                    functions,
                    examples: Vec::new(),
                },
                source,
                enums: renamer.enums,
                callees: callee_names,
                callee_bodies,
                hash,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{hash, units};
    use crate::{check, obligation, syntax};

    /// Each function's name, its definition hash and its obligations' keys.
    fn identities(source: &str) -> Vec<(String, String, Vec<String>)> {
        let mut program = syntax::parse(source).expect("parses");
        check::check(&mut program, &[]).expect("checks");
        units(&program)
            .iter()
            .map(|unit| {
                let keys = obligation::unit_obligations(&unit.program)
                    .iter()
                    .map(|o| hash(&o.script))
                    .collect();
                (unit.source.name.clone(), unit.hash.clone(), keys)
            })
            .collect()
    }

    fn without_names(source: &str) -> Vec<(String, Vec<String>)> {
        identities(source)
            .into_iter()
            .map(|(_, hash, keys)| (hash, keys))
            .collect()
    }

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

    #[test]
    fn no_rename_changes_any_identity() {
        let renamed = "// Every name differs; nothing it means does.
enum Signal { Stop, Wait, Go }

fn advance(s: Signal) -> Signal
    intent \"Moves to the next signal.\"
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
    x + y
}
";
        assert_eq!(without_names(ORIGINAL), without_names(renamed));
    }

    const CALLS: &str = "fn successor(n: i32) -> i32
    requires n < 100
    ensures result == n + 1
{
    n + 1
}

fn twice(n: i32) -> i32
    requires n >= 0 and n < 50
    ensures result == n + 2
{
    successor(successor(n))
}
";

    #[test]
    fn a_callee_body_change_leaves_its_callers_identity() {
        let changed = CALLS.replace("    n + 1\n}", "    1 + n\n}");
        let (before, after) = (identities(CALLS), identities(&changed));
        assert_ne!(before[0].1, after[0].1, "`successor`'s definition changed");
        assert_eq!(before[1], after[1], "`twice` relies only on its contract");
        let bodies = |source: &str| {
            let mut program = syntax::parse(source).expect("parses");
            check::check(&mut program, &[]).expect("checks");
            units(&program)[1].callee_bodies.clone()
        };
        assert_ne!(
            bodies(CALLS),
            bodies(&changed),
            "what a replayed model depends on did change"
        );
    }

    #[test]
    fn a_callee_contract_change_changes_its_callers_identity() {
        let weaker = CALLS.replace("ensures result == n + 1", "ensures result > n");
        let (before, after) = (identities(CALLS), identities(&weaker));
        assert_ne!(before[1].1, after[1].1);
        assert_ne!(before[1].2, after[1].2);
    }

    #[test]
    fn renaming_a_callee_changes_no_identity() {
        let renamed = CALLS.replace("successor", "next");
        assert_eq!(without_names(CALLS), without_names(&renamed));
    }

    #[test]
    fn a_body_change_changes_only_its_own_function() {
        let changed = ORIGINAL.replace("    a + b\n}", "    b + a\n}");
        let (before, after) = (identities(ORIGINAL), identities(&changed));
        assert_eq!(before[0], after[0], "`next` is untouched");
        assert_ne!(before[1].1, after[1].1, "`add`'s definition changed");
        assert_ne!(before[1].2, after[1].2, "and so did its evidence keys");
    }

    #[test]
    fn an_enum_change_reaches_only_the_functions_that_use_it() {
        let wider = ORIGINAL.replace("Green }", "Green, Off }");
        let (before, after) = (identities(ORIGINAL), identities(&wider));
        assert_ne!(before[0].1, after[0].1, "`next` uses `Light`");
        assert_eq!(before[1], after[1], "`add` doesn't");
        let unrelated = format!("enum Unused {{ A, B }}\n{ORIGINAL}");
        assert_eq!(
            identities(&unrelated),
            before,
            "an unused enum changes nothing"
        );
    }

    #[test]
    fn variant_order_is_part_of_the_meaning() {
        let reordered = ORIGINAL.replace("{ Red, Amber, Green }", "{ Amber, Red, Green }");
        assert_ne!(identities(ORIGINAL)[0].1, identities(&reordered)[0].1);
    }

    #[test]
    fn a_contract_change_changes_the_definition() {
        let weaker = ORIGINAL.replace("ensures result == a + b", "ensures result >= a + b");
        assert_ne!(identities(ORIGINAL)[1].1, identities(&weaker)[1].1);
        // A declared freedom is part of what the specification means.
        let free = ORIGINAL.replace(
            "ensures result == a + b\n",
            "ensures result == a + b\n    open when a > 0\n",
        );
        assert_ne!(identities(ORIGINAL)[1].1, identities(&free)[1].1);
    }
}
