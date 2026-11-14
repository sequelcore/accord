//! Reading what the solvers say about a normalized function back in
//! source terms, for people.

use super::Unit;
use crate::interpret::{self, Value};
use crate::syntax::Type;
use crate::{Inputs, Outcome, Refutation};

impl Unit<'_> {
    /// Source names and variants of the enums this unit uses.
    pub fn source_enums(&self) -> Vec<(String, Vec<String>)> {
        self.enums
            .iter()
            .map(|e| (e.name.clone(), e.variants.clone()))
            .collect()
    }

    /// A solver's inputs for the normalized function, in source terms.
    pub fn source_inputs(&self, inputs: &Inputs) -> Inputs {
        inputs
            .iter()
            .map(|(name, value)| {
                let index = name.strip_prefix('p').and_then(|i| i.parse::<usize>().ok());
                let source_name = index
                    .and_then(|i| self.source.params.get(i))
                    .map_or(name.clone(), |(source, _)| source.clone());
                let source_value = self
                    .program
                    .functions
                    .first()
                    .and_then(|f| index.and_then(|i| f.params.get(i)))
                    .map_or_else(
                        || value.clone(),
                        |(_, ty)| self.source_typed_value(value, ty),
                    );
                (source_name, source_value)
            })
            .collect()
    }

    /// A value of the normalized function in source terms.
    pub fn source_of(&self, value: &Value) -> String {
        self.source_value_tree(value.clone()).source()
    }

    /// A normalized value, such as `E0.v1`, in source terms.
    pub fn source_value(&self, value: &str) -> String {
        self.program.functions.first().map_or_else(
            || value.to_string(),
            |f| self.source_typed_value(value, &f.ret),
        )
    }

    /// A normalized value of type `ty`, in source terms.
    pub fn source_typed_value(&self, value: &str, ty: &Type) -> String {
        interpret::read(value, ty)
            .map_or_else(|| value.to_string(), |v| self.source_value_tree(v).source())
    }

    fn source_value_tree(&self, value: Value) -> Value {
        match value {
            Value::List(items) => Value::List(
                items
                    .into_iter()
                    .map(|v| self.source_value_tree(v))
                    .collect(),
            ),
            Value::Variant(owner, variant, fields) => {
                let fields = self.source_fields(fields);
                let source = (|| {
                    let e = self.enums.get(position(&owner, 'E')?)?;
                    let index = position(&variant, 'v')?;
                    let name = e.variants.get(index)?;
                    let labels = relabel(&fields, &e.fields[index])?;
                    Some(Value::Variant(e.name.clone(), name.clone(), labels))
                })();
                source.unwrap_or(Value::Variant(owner, variant, fields))
            }
            Value::Record(name, fields) => {
                let fields = self.source_fields(fields);
                let source = (|| {
                    let r = self.records.get(position(&name, 'R')?)?;
                    Some(Value::Record(r.name.clone(), relabel(&fields, &r.fields)?))
                })();
                source.unwrap_or(Value::Record(name, fields))
            }
            other => other,
        }
    }

    fn source_fields(&self, fields: Vec<(String, Value)>) -> Vec<(String, Value)> {
        fields
            .into_iter()
            .map(|(name, value)| (name, self.source_value_tree(value)))
            .collect()
    }
}

/// The index in a normalized name such as `E2`, after its `prefix`.
fn position(name: &str, prefix: char) -> Option<usize> {
    name.strip_prefix(prefix)?.parse().ok()
}

/// Normalized fields, `x0`, `x1`, ..., under their source names.
fn relabel(
    fields: &[(String, Value)],
    declared: &[(String, Type)],
) -> Option<Vec<(String, Value)>> {
    fields
        .iter()
        .map(|(name, value)| {
            let (source, _) = declared.get(position(name, 'x')?)?;
            Some((source.clone(), value.clone()))
        })
        .collect()
}

impl Unit<'_> {
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
            Outcome::WeakCallee { inputs, calls } => Outcome::WeakCallee {
                inputs: self.source_inputs(&inputs),
                calls,
            },
            other => other,
        }
    }
}
