//! Reading a solver's reply: whether its assertions can hold, and the values
//! `get-value` gives, in Accord source.

/// What one solver said about one obligation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Answer {
    /// The script's assertions can hold, with the parameters' values.
    Sat(Vec<(String, String)>),
    /// They can't.
    Unsat,
    /// No answer, with the reason.
    Unknown(String),
}

/// Reads a solver's reply: its first line, then the `get-value` list on `sat`.
pub(crate) fn parse_answer(output: &str, params: &[String]) -> Answer {
    let mut lines = output.lines();
    match lines.next().map(str::trim) {
        Some("unsat") => Answer::Unsat,
        Some("sat") => {
            let rest: String = lines.collect::<Vec<_>>().join(" ");
            let values = params
                .iter()
                .filter_map(|name| value_of(&rest, name).map(|value| (name.clone(), value)))
                .collect::<Vec<_>>();
            if values.len() == params.len() {
                Answer::Sat(values)
            } else {
                Answer::Unknown("sat, but the counterexample couldn't be read".to_string())
            }
        }
        Some("unknown") => {
            let rest: String = lines.collect::<Vec<_>>().join(" ");
            let reason = rest
                .split_once("(:reason-unknown ")
                .and_then(|(_, r)| r.split(')').next())
                .map(|r| r.trim().trim_matches('"'));
            Answer::Unknown(match reason {
                Some("timeout") => "the solver ran out of time".to_string(),
                Some(reason) if !reason.is_empty() => {
                    format!("the solver answered unknown ({reason})")
                }
                _ => "the solver answered unknown".to_string(),
            })
        }
        Some("timeout") => Answer::Unknown("the solver ran out of time".to_string()),
        _ => Answer::Unknown("the solver gave no answer".to_string()),
    }
}

/// One SMT-LIB value as `get-value` prints it.
enum Sexp {
    Atom(String),
    /// A string literal's characters, decoded.
    Str(String),
    List(Vec<Sexp>),
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn parse_sexp(chars: &mut Chars) -> Option<Sexp> {
    while chars.peek()?.is_whitespace() {
        chars.next();
    }
    match chars.next()? {
        '(' => {
            let mut items = Vec::new();
            loop {
                while chars.peek()?.is_whitespace() {
                    chars.next();
                }
                if chars.peek() == Some(&')') {
                    chars.next();
                    return Some(Sexp::List(items));
                }
                items.push(parse_sexp(chars)?);
            }
        }
        '"' => read_string(chars).map(Sexp::Str),
        first => {
            let mut atom = String::from(first);
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '(' || c == ')' {
                    break;
                }
                atom.push(c);
                chars.next();
            }
            Some(Sexp::Atom(atom))
        }
    }
}

/// An SMT-LIB string literal's characters, after its opening quote: `""` is
/// a quote, and `\u{...}` or `\uXXXX` a character. Z3 prints a backslash
/// raw, so a text that itself contains `\u{...}` reads back as the character
/// it spells; the interpreter's replay then rejects the model, and the
/// obligation is undecided, never wrongly decided.
fn read_string(chars: &mut Chars) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' if chars.peek() == Some(&'"') => {
                chars.next();
                out.push('"');
            }
            '"' => return Some(out),
            '\\' if chars.peek() == Some(&'u') => {
                chars.next();
                let hex: String = if chars.peek() == Some(&'{') {
                    chars.next();
                    chars.by_ref().take_while(|c| *c != '}').collect()
                } else {
                    chars.by_ref().take(4).collect()
                };
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            c => out.push(c),
        }
    }
}

/// A sequence value's elements, in Accord source: `(seq.unit x)`,
/// `(seq.++ ...)` and `(as seq.empty ...)`.
fn elements(value: &Sexp) -> Option<Vec<String>> {
    let Sexp::List(items) = value else {
        return None;
    };
    match items.as_slice() {
        [Sexp::Atom(f), x] if f == "seq.unit" => Some(vec![source(x)?]),
        [Sexp::Atom(f), parts @ ..] if f == "seq.++" => {
            let mut out = Vec::new();
            for part in parts {
                out.extend(elements(part)?);
            }
            Some(out)
        }
        [Sexp::Atom(a), Sexp::Atom(e), _] if a == "as" && e == "seq.empty" => Some(Vec::new()),
        _ => None,
    }
}

/// A value in Accord source: a symbol or number as printed, `(- n)` as `-n`,
/// a string in quotes, a sequence as a list literal, and a constructor with
/// fields as a variant's or a record's value. Scripts are written from
/// normalized programs, whose fields are `x0`, `x1`, ... by position, and
/// whose variants, unlike records, have a `.` in their constructor's name.
fn source(value: &Sexp) -> Option<String> {
    match value {
        Sexp::Atom(atom) => Some(atom.clone()),
        Sexp::Str(text) => Some(format!("\"{text}\"")),
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Atom(minus), Sexp::Atom(n)] if minus == "-" => Some(format!("-{n}")),
            [Sexp::Atom(f), ..] if f.starts_with("seq.") || f == "as" => {
                elements(value).map(|items| format!("[{}]", items.join(", ")))
            }
            [Sexp::Atom(constructor), fields @ ..] if !fields.is_empty() => {
                let fields = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| Some(format!("x{index}: {}", source(field)?)))
                    .collect::<Option<Vec<_>>>()?
                    .join(", ");
                Some(if constructor.contains('.') {
                    format!("{constructor}({fields})")
                } else {
                    format!("{constructor} {{ {fields} }}")
                })
            }
            _ => None,
        },
    }
}

/// The value `get-value` gives `name`, in Accord source.
fn value_of(text: &str, name: &str) -> Option<String> {
    let start = text.find(&format!("({name} "))? + name.len() + 2;
    source(&parse_sexp(&mut text[start..].chars().peekable())?)
}

#[cfg(test)]
mod tests {
    use super::{Answer, parse_answer};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_timeout_or_garbage_is_unknown() {
        assert!(matches!(parse_answer("timeout\n", &[]), Answer::Unknown(_)));
        assert!(matches!(parse_answer("unknown\n", &[]), Answer::Unknown(_)));
        assert!(matches!(
            parse_answer("(error \"bad\")\n", &[]),
            Answer::Unknown(_)
        ));
        assert!(matches!(parse_answer("", &[]), Answer::Unknown(_)));
    }

    #[test]
    fn sat_needs_a_value_for_every_parameter() {
        let both = parse_answer("sat\n((a true)\n (b false))\n", &names(&["a", "b"]));
        assert_eq!(
            both,
            Answer::Sat(vec![
                ("a".to_string(), "true".to_string()),
                ("b".to_string(), "false".to_string())
            ])
        );
        assert!(matches!(
            parse_answer("sat\n((a true))\n", &names(&["a", "b"])),
            Answer::Unknown(_)
        ));
    }

    #[test]
    fn an_unknown_answer_says_when_the_solver_ran_out_of_time() {
        let z3 = "unknown\n(error \"model is not available\")\n(:reason-unknown \"timeout\")\n";
        let cvc5 = "unknown\n((a 0))\n(:reason-unknown timeout)\n";
        for output in [z3, cvc5] {
            assert_eq!(
                parse_answer(output, &names(&["a"])),
                Answer::Unknown("the solver ran out of time".to_string())
            );
        }
        assert_eq!(
            parse_answer("unknown\n(:reason-unknown incomplete)\n", &[]),
            Answer::Unknown("the solver answered unknown (incomplete)".to_string())
        );
    }

    #[test]
    fn text_values_read_as_source() {
        let both = parse_answer(
            "sat\n((a \"caf\\u{e9}\")\n (b \"say \"\"hi\"\"\"))\n",
            &names(&["a", "b"]),
        );
        assert_eq!(
            both,
            Answer::Sat(vec![
                ("a".to_string(), "\"café\"".to_string()),
                ("b".to_string(), "\"say \"hi\"\"".to_string())
            ])
        );
    }

    #[test]
    fn a_raw_backslash_from_z3_reads_as_itself() {
        // Z3 prints a backslash raw where cvc5 writes `\u{5c}`; both read alike.
        for output in ["sat\n((a \"x \\\"))\n", "sat\n((a \"x \\u{5c}\"))\n"] {
            assert_eq!(
                parse_answer(output, &names(&["a"])),
                Answer::Sat(vec![("a".to_string(), "\"x \\\"".to_string())])
            );
        }
    }

    #[test]
    fn sequences_read_as_list_literals() {
        // Both solvers' forms, with a `)` inside a text and an empty list.
        let output = "sat\n((s (seq.++ (seq.unit \"x)\") (seq.unit \"caf\\u{e9}\")))\n (c (seq.unit E0.v1))\n (e (as seq.empty (Seq String))))\n";
        assert_eq!(
            parse_answer(output, &names(&["s", "c", "e"])),
            Answer::Sat(vec![
                ("s".to_string(), "[\"x)\", \"café\"]".to_string()),
                ("c".to_string(), "[E0.v1]".to_string()),
                ("e".to_string(), "[]".to_string())
            ])
        );
    }

    #[test]
    fn variants_and_records_read_as_source() {
        // Z3's form, then cvc5's, with a field that is itself a sequence.
        let output = "sat\n((p0 (R0 4 (seq.++ (seq.unit (E0.v0 \"!0!\")) (seq.unit E0.v2))))\n (p1 (E0.v1 (- 1) 0)))\n";
        assert_eq!(
            parse_answer(output, &names(&["p0", "p1"])),
            Answer::Sat(vec![
                (
                    "p0".to_string(),
                    "R0 { x0: 4, x1: [E0.v0(x0: \"!0!\"), E0.v2] }".to_string()
                ),
                ("p1".to_string(), "E0.v1(x0: -1, x1: 0)".to_string())
            ])
        );
    }

    #[test]
    fn negative_values_read_as_source() {
        assert_eq!(
            parse_answer("sat\n((a (- 2147483648))\n (b 7))\n", &names(&["a", "b"])),
            Answer::Sat(vec![
                ("a".to_string(), "-2147483648".to_string()),
                ("b".to_string(), "7".to_string())
            ])
        );
    }
}
