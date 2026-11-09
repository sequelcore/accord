//! A definition's identity is its meaning, not its names. Each function is
//! normalized before any obligation is generated: parameters become `p0`,
//! `p1`, ..., the enums it uses become `E0`, `E1`, ... in order of first use,
//! and their variants `v0`, `v1`, ... by position. Obligations come from the
//! normalized function, so a rename changes no script, and a script declares
//! only the enums its function uses.
//!
//! What the solvers say about a normalized function is translated back to
//! source names only for people to read.

use sha2::{Digest, Sha256};

use crate::syntax::{Body, Enum, Expr, Function, Program, Type};
use crate::{Inputs, Outcome, Refutation};

/// One function, normalized, with what's needed to read results back.
pub(crate) struct Unit<'a> {
    /// The normalized function and the enums it uses, as a program of its own.
    pub program: Program,
    /// The function as written.
    pub source: &'a Function,
    /// Each normalized enum's source declaration, by index.
    enums: Vec<&'a Enum>,
    /// The hash of the normalized function: its definition's identity.
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

struct Renamer<'a> {
    program: &'a Program,
    params: &'a [(String, Type)],
    enums: Vec<&'a Enum>,
}

impl<'a> Renamer<'a> {
    fn enum_index(&mut self, name: &str) -> usize {
        if let Some(index) = self.enums.iter().position(|e| e.name == name) {
            return index;
        }
        let Some(e) = self.program.enums.iter().find(|e| e.name == name) else {
            unreachable!("`check` rejects the unknown enum `{name}`")
        };
        self.enums.push(e);
        self.enums.len() - 1
    }

    fn ty(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Enum(name) => Type::Enum(format!("E{}", self.enum_index(name))),
            Type::List(element) => Type::List(Box::new(self.ty(element))),
            other => other.clone(),
        }
    }

    fn expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Name(name, _) if name != "result" => {
                if let Some(index) = self.params.iter().position(|(n, _)| n == name) {
                    *name = format!("p{index}");
                }
            }
            Expr::Variant(owner, variant, _) => {
                let index = self.enum_index(owner);
                let position = self.enums[index]
                    .variants
                    .iter()
                    .position(|v| v == variant)
                    .unwrap_or_default();
                *owner = format!("E{index}");
                *variant = format!("v{position}");
            }
            Expr::Not(inner) | Expr::Neg(inner, _) => self.expr(inner),
            Expr::Binary(_, left, right) | Expr::Arith(_, left, right, _) => {
                self.expr(left);
                self.expr(right);
            }
            Expr::If(condition, then, otherwise) => {
                self.expr(condition);
                self.expr(then);
                self.expr(otherwise);
            }
            Expr::List(items, element, _) => {
                for item in items {
                    self.expr(item);
                }
                if let Some(t) = element {
                    *t = self.ty(t);
                }
            }
            Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) => {}
        }
    }
}

/// The canonical text of a normalized function: everything it means and
/// nothing about where or how it is written.
fn canonical(function: &Function, enums: &[Enum]) -> String {
    let mut out = String::from("(fn");
    for e in enums {
        out += &format!(" (enum {} {})", e.name, e.variants.len());
    }
    for (_, ty) in &function.params {
        out += &format!(" (param {ty:?})");
    }
    out += &format!(" (returns {:?})", function.ret);
    for clause in &function.clauses {
        out += &format!(
            " ({:?} {})",
            clause.kind,
            crate::obligation::term(&clause.expr)
        );
    }
    match &function.body {
        Body::Expr(body) => out += &format!(" (body {})", crate::obligation::term(body)),
        Body::Decide(rows, _) => {
            out += " (decide";
            for row in rows {
                let condition = row
                    .condition
                    .as_ref()
                    .map_or("otherwise".to_string(), crate::obligation::term);
                out += &format!(" ({condition} {})", crate::obligation::term(&row.result));
            }
            out += ")";
        }
    }
    out + ")"
}

/// Every function of a checked program, normalized, in source order.
pub(crate) fn units(program: &Program) -> Vec<Unit<'_>> {
    program
        .functions
        .iter()
        .map(|source| {
            let mut function = source.clone();
            let mut renamer = Renamer {
                program,
                params: &source.params,
                enums: Vec::new(),
            };
            function.params = source
                .params
                .iter()
                .enumerate()
                .map(|(index, (_, ty))| (format!("p{index}"), renamer.ty(ty)))
                .collect();
            function.ret = renamer.ty(&source.ret);
            for clause in &mut function.clauses {
                renamer.expr(&mut clause.expr);
            }
            match &mut function.body {
                Body::Expr(body) => renamer.expr(body),
                Body::Decide(rows, _) => {
                    for row in rows {
                        if let Some(condition) = &mut row.condition {
                            renamer.expr(condition);
                        }
                        renamer.expr(&mut row.result);
                    }
                }
            }
            let enums: Vec<Enum> = renamer
                .enums
                .iter()
                .enumerate()
                .map(|(index, e)| Enum {
                    name: format!("E{index}"),
                    variants: (0..e.variants.len()).map(|v| format!("v{v}")).collect(),
                    line: 0,
                    file: 0,
                })
                .collect();
            let hash = hash(&canonical(&function, &enums));
            Unit {
                program: Program {
                    enums,
                    functions: vec![function],
                    examples: Vec::new(),
                },
                source,
                enums: renamer.enums,
                hash,
            }
        })
        .collect()
}

impl Unit<'_> {
    /// A solver's inputs for the normalized function, in source terms.
    pub fn source_inputs(&self, inputs: &Inputs) -> Inputs {
        inputs
            .iter()
            .map(|(name, value)| {
                let name = name
                    .strip_prefix('p')
                    .and_then(|i| i.parse::<usize>().ok())
                    .and_then(|i| self.source.params.get(i))
                    .map_or(name.clone(), |(source, _)| source.clone());
                (name, self.source_value(value))
            })
            .collect()
    }

    /// A normalized value, such as `E0.v1`, in source terms.
    pub fn source_value(&self, value: &str) -> String {
        let read = || {
            let (owner, variant) = value.strip_prefix('E')?.split_once(".v")?;
            let e = self.enums.get(owner.parse::<usize>().ok()?)?;
            let name = e.variants.get(variant.parse::<usize>().ok()?)?;
            Some(format!("{}.{name}", e.name))
        };
        read().unwrap_or_else(|| value.to_string())
    }

    /// An outcome about the normalized function, in source terms.
    pub fn source_outcome(&self, outcome: Outcome) -> Outcome {
        match outcome {
            Outcome::Proved {
                witness: Some(inputs),
            } => Outcome::Proved {
                witness: Some(self.source_inputs(&inputs)),
            },
            Outcome::Refuted {
                evidence: Refutation::Counterexample(inputs),
            } => Outcome::Refuted {
                evidence: Refutation::Counterexample(self.source_inputs(&inputs)),
            },
            other => other,
        }
    }
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
                let keys = obligation::obligations(&unit.program)
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
