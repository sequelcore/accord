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
            Value::Variant(owner, variant) => {
                let source = (|| {
                    let e = self
                        .enums
                        .get(owner.strip_prefix('E')?.parse::<usize>().ok()?)?;
                    let name = e
                        .variants
                        .get(variant.strip_prefix('v')?.parse::<usize>().ok()?)?;
                    Some(Value::Variant(e.name.clone(), name.clone()))
                })();
                source.unwrap_or(Value::Variant(owner, variant))
            }
            other => other,
        }
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
            Outcome::WeakCallee { inputs, calls } => Outcome::WeakCallee {
                inputs: self.source_inputs(&inputs),
                calls,
            },
            other => other,
        }
    }
}
