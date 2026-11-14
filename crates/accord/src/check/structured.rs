//! Structured data: variants built with their fields, records, fields and
//! `match`.

use super::calls::ordered;
use super::expr::Scope;
use super::type_name;
use crate::Diagnostic;
use crate::syntax::{Arm, Binding, Expr, Owner, Type};

impl Scope<'_> {
    /// Rejects a bare or qualified variant that carries fields.
    pub(super) fn fieldless(
        &self,
        owner: &str,
        variant: &str,
        line: usize,
    ) -> Result<(), Diagnostic> {
        let carries = self
            .types
            .enumeration(owner)
            .is_some_and(|e| !e.fields_of(variant).is_empty());
        if carries {
            return Err(Diagnostic::new(
                line,
                format!("`{variant}` has fields: build it as `{variant}(…)`"),
            ));
        }
        Ok(())
    }

    /// The type of a variant built with its fields, which it puts in their
    /// declared order, each labelled.
    pub(super) fn construct(
        &self,
        owner: &str,
        variant: &str,
        args: &mut Vec<(Option<String>, Expr)>,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let Some(e) = self.types.enumeration(owner) else {
            return Err(Diagnostic::new(line, format!("unknown type `{owner}`")));
        };
        if !e.variants.iter().any(|v| v == variant) {
            return Err(Diagnostic::new(
                line,
                format!("`{owner}` has no variant `{variant}`"),
            ));
        }
        let fields = e.fields_of(variant);
        if fields.is_empty() {
            return Err(Diagnostic::new(
                line,
                format!("`{variant}` has no fields: write it without parentheses"),
            ));
        }
        if args.len() != fields.len() {
            return Err(Diagnostic::new(
                line,
                format!("`{variant}` has {} fields: give each", fields.len()),
            ));
        }
        if fields.len() > 1 && args.iter().any(|(label, _)| label.is_none()) {
            return Err(Diagnostic::new(
                line,
                format!("label every field: `{variant}` has more than one"),
            ));
        }
        if let Some(label) = args
            .iter()
            .filter_map(|(label, _)| label.as_ref())
            .find(|label| !fields.iter().any(|(f, _)| f == *label))
        {
            return Err(Diagnostic::new(
                line,
                format!("`{variant}` has no field `{label}`"),
            ));
        }
        let given = ordered(variant, fields, std::mem::take(args), line)?;
        for ((field, ty), mut value) in fields.iter().zip(given) {
            self.value(
                &mut value,
                ty,
                &format!("`{field}` needs a `{}`", type_name(ty)),
            )?;
            args.push((Some(field.clone()), value));
        }
        Ok(Type::Enum(owner.to_string()))
    }

    /// The type of `Name { field: value, ... }`, which it puts in the
    /// fields' declared order. Every field is named once.
    pub(super) fn record_value(
        &self,
        name: &str,
        fields: &mut Vec<(String, Expr)>,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let Some(record) = self.types.record(name) else {
            return Err(Diagnostic::new(line, format!("unknown record `{name}`")));
        };
        let mut values: Vec<Option<Expr>> = record.fields.iter().map(|_| None).collect();
        for (field, value) in std::mem::take(fields) {
            let Some(index) = record.fields.iter().position(|(f, _)| *f == field) else {
                return Err(Diagnostic::new(
                    line,
                    format!("`{name}` has no field `{field}`"),
                ));
            };
            if values[index].replace(value).is_some() {
                return Err(Diagnostic::new(line, format!("`{field}` is given twice")));
            }
        }
        for ((field, ty), value) in record.fields.iter().zip(values) {
            let Some(mut value) = value else {
                return Err(Diagnostic::new(
                    line,
                    format!("`{field}` is missing: a record's value names every field"),
                ));
            };
            self.value(
                &mut value,
                ty,
                &format!("`{field}` needs a `{}`", type_name(ty)),
            )?;
            fields.push((field.clone(), value));
        }
        Ok(Type::Record(name.to_string()))
    }

    /// The type of `value.field`, on a record.
    pub(super) fn field(
        &self,
        value: &mut Expr,
        owner: &mut Option<Owner>,
        field: &str,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let fields = match owner {
            Some(Owner::Variant(e, variant)) => self
                .types
                .enumeration(e)
                .map(|e| e.fields_of(variant).to_vec()),
            Some(Owner::Record(name)) => self.types.record(name).map(|r| r.fields.clone()),
            None => match self.infer(value, None)? {
                Type::Record(name) => {
                    let fields = self.types.record(&name).map(|r| r.fields.clone());
                    *owner = Some(Owner::Record(name));
                    fields
                }
                Type::Enum(name) => {
                    return Err(Diagnostic::new(
                        line,
                        format!("a `{name}`'s fields are read with `match`"),
                    ));
                }
                other => {
                    return Err(Diagnostic::new(
                        line,
                        format!("only a record has fields, not a `{}`", type_name(&other)),
                    ));
                }
            },
        };
        let found = fields
            .unwrap_or_default()
            .into_iter()
            .find(|(f, _)| f == field)
            .map(|(_, ty)| ty);
        found.ok_or_else(|| match owner {
            Some(Owner::Record(name)) => {
                Diagnostic::new(line, format!("`{name}` has no field `{field}`"))
            }
            _ => Diagnostic::new(line, format!("no field `{field}` here")),
        })
    }

    /// The type of a `match`: every variant of the value's enum, each once,
    /// with each arm's names bound to its fields. Each name becomes the
    /// field it stands for, and each arm gets the condition under which it
    /// runs.
    pub(super) fn matched(
        &self,
        value: &mut Expr,
        arms: &mut [Arm],
        expected: Option<&Type>,
        line: usize,
    ) -> Result<Type, Diagnostic> {
        let owner = match self.infer(value, None)? {
            Type::Enum(owner) => owner,
            other => {
                return Err(Diagnostic::new(
                    line,
                    format!("only an enum can be matched, not a `{}`", type_name(&other)),
                ));
            }
        };
        let Some(e) = self.types.enumeration(&owner) else {
            return Err(Diagnostic::new(line, format!("unknown type `{owner}`")));
        };
        let mut result: Option<Type> = expected.cloned();
        let mut seen: Vec<&str> = Vec::new();
        for arm in arms.iter_mut() {
            if !e.variants.contains(&arm.variant) {
                return Err(Diagnostic::new(
                    arm.line,
                    format!("`{owner}` has no variant `{}`", arm.variant),
                ));
            }
            if seen.contains(&arm.variant.as_str()) {
                return Err(Diagnostic::new(
                    arm.line,
                    format!("`{}` is matched twice", arm.variant),
                ));
            }
            seen.push(
                e.variants
                    .iter()
                    .find(|v| **v == arm.variant)
                    .map_or("", String::as_str),
            );
            let fields = e.fields_of(&arm.variant);
            let names = self.bindings(&arm.variant, fields, &mut arm.bindings, arm.line)?;
            let mut params = self.params.to_vec();
            params.extend(names.iter().map(|(name, ty, _)| (name.clone(), ty.clone())));
            let inner = Scope {
                params: &params,
                ..*self
            };
            let ty = inner.infer(&mut arm.result, result.as_ref())?;
            match &result {
                Some(want) if *want != ty => {
                    return Err(Diagnostic::new(
                        arm.line,
                        "every arm of a `match` needs the same type",
                    ));
                }
                _ => result = Some(ty),
            }
            for (name, _, field) in &names {
                let read = Expr::Field(
                    Box::new(value.clone()),
                    Some(Owner::Variant(owner.clone(), arm.variant.clone())),
                    field.clone(),
                    arm.line,
                );
                substitute(&mut arm.result, name, &read);
            }
            arm.test = Some(Expr::Is(
                Box::new(value.clone()),
                owner.clone(),
                arm.variant.clone(),
            ));
        }
        if let Some(missing) = e.variants.iter().find(|v| !seen.contains(&v.as_str())) {
            return Err(Diagnostic::new(
                line,
                format!("the `match` doesn't cover `{missing}` of `{owner}`"),
            ));
        }
        result.ok_or_else(|| Diagnostic::new(line, "a `match` needs an arm"))
    }

    /// Checks a pattern's names against its variant's fields and puts them
    /// in the fields' order, each labelled: by position for one field, by
    /// label for more. Gives each name with its field's type and the field.
    fn bindings(
        &self,
        variant: &str,
        fields: &[(String, Type)],
        bindings: &mut Vec<Binding>,
        line: usize,
    ) -> Result<Vec<(String, Type, String)>, Diagnostic> {
        if fields.is_empty() && !bindings.is_empty() {
            return Err(Diagnostic::new(
                line,
                format!("`{variant}` has no fields to bind"),
            ));
        }
        if !fields.is_empty() && bindings.len() != fields.len() {
            return Err(Diagnostic::new(
                line,
                format!(
                    "`{variant}` has {} fields: give each a name, or `_`",
                    fields.len()
                ),
            ));
        }
        if fields.len() > 1 && bindings.iter().any(|(label, _)| label.is_none()) {
            return Err(Diagnostic::new(
                line,
                format!("label every field: `{variant}` has more than one"),
            ));
        }
        let mut ordered: Vec<Option<Option<String>>> = fields.iter().map(|_| None).collect();
        for (label, name) in std::mem::take(bindings) {
            let index = match &label {
                None => 0,
                Some(label) => fields.iter().position(|(f, _)| f == label).ok_or_else(|| {
                    Diagnostic::new(line, format!("`{variant}` has no field `{label}`"))
                })?,
            };
            if ordered[index].replace(name).is_some() {
                return Err(Diagnostic::new(
                    line,
                    format!("`{}` is bound twice", fields[index].0),
                ));
            }
        }
        let mut names: Vec<(String, Type, String)> = Vec::new();
        for ((field, ty), name) in fields.iter().zip(ordered) {
            let name = name.flatten();
            if let Some(name) = &name {
                if self.param(name).is_some() || names.iter().any(|(n, ..)| n == name) {
                    return Err(Diagnostic::new(
                        line,
                        format!("`{name}` is already a name here"),
                    ));
                }
                names.push((name.clone(), ty.clone(), field.clone()));
            }
            bindings.push((Some(field.clone()), name));
        }
        Ok(names)
    }
}

/// `expr` with every `name` replaced by `with`. No binding hides a name in
/// scope, so every `name` inside is the one bound.
fn substitute(expr: &mut Expr, name: &str, with: &Expr) {
    if matches!(expr, Expr::Name(n, _) if n == name) {
        *expr = with.clone();
        return;
    }
    for child in expr.children_mut() {
        substitute(child, name, with);
    }
}
