//! Normalizing a function: its names replaced by positions, and the
//! canonical text its identity hashes.

use crate::syntax::{Body, ClauseKind, Enum, Expr, Function, Owner, Program, Record, Type};

/// The name of the field at `index`, normalized.
fn field_name(index: usize) -> String {
    format!("x{index}")
}

/// The position of `name` among `fields`.
fn position(fields: &[(String, Type)], name: &str) -> usize {
    fields
        .iter()
        .position(|(f, _)| f == name)
        .unwrap_or_default()
}

pub(super) struct Renamer<'a> {
    pub(super) program: &'a Program,
    pub(super) enums: Vec<&'a Enum>,
    pub(super) records: Vec<&'a Record>,
    pub(super) functions: Vec<&'a Function>,
}

impl<'a> Renamer<'a> {
    /// The enum's position, found or added; the types its fields hold are
    /// added with it.
    fn enum_index(&mut self, name: &str) -> usize {
        if let Some(index) = self.enums.iter().position(|e| e.name == name) {
            return index;
        }
        let Some(e) = self.program.enums.iter().find(|e| e.name == name) else {
            unreachable!("`check` rejects the unknown enum `{name}`")
        };
        self.enums.push(e);
        for (_, ty) in e.fields.iter().flatten() {
            self.ty(ty);
        }
        self.enums
            .iter()
            .position(|o| o.name == name)
            .unwrap_or_default()
    }

    /// The record's position, found or added; the types its fields hold are
    /// added with it.
    fn record_index(&mut self, name: &str) -> usize {
        if let Some(index) = self.records.iter().position(|r| r.name == name) {
            return index;
        }
        let Some(r) = self.program.records.iter().find(|r| r.name == name) else {
            unreachable!("`check` rejects the unknown record `{name}`")
        };
        self.records.push(r);
        for (_, ty) in &r.fields {
            self.ty(ty);
        }
        self.records
            .iter()
            .position(|o| o.name == name)
            .unwrap_or_default()
    }

    /// The enum at `index` with its names replaced by positions: `E{index}`,
    /// with variants `v0`, `v1`, ..., fields `x0`, `x1`, ... and no written
    /// values, which no proof reads.
    pub(super) fn enumeration(&mut self, index: usize) -> Enum {
        let e = self.enums[index];
        Enum {
            name: format!("E{index}"),
            variants: (0..e.variants.len()).map(|v| format!("v{v}")).collect(),
            values: vec![None; e.variants.len()],
            fields: e.fields.iter().map(|fields| self.fields(fields)).collect(),
            line: 0,
            file: 0,
        }
    }

    /// The record at `index` with its names replaced by positions.
    pub(super) fn record(&mut self, index: usize) -> Record {
        let r = self.records[index];
        Record {
            name: format!("R{index}"),
            fields: self.fields(&r.fields),
            line: 0,
            file: 0,
        }
    }

    fn fields(&mut self, fields: &[(String, Type)]) -> Vec<(String, Type)> {
        fields
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| (field_name(index), self.ty(ty)))
            .collect()
    }

    fn function_index(&mut self, name: &str) -> usize {
        if let Some(index) = self.functions.iter().position(|f| f.name == name) {
            return index;
        }
        let Some(f) = self.program.functions.iter().find(|f| f.name == name) else {
            unreachable!("`check` rejects the unknown function `{name}`")
        };
        self.functions.push(f);
        self.functions.len() - 1
    }

    fn ty(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Enum(name) => Type::Enum(format!("E{}", self.enum_index(name))),
            Type::Record(name) => Type::Record(format!("R{}", self.record_index(name))),
            Type::List(element) => Type::List(Box::new(self.ty(element))),
            other => other.clone(),
        }
    }

    fn expr(&mut self, expr: &mut Expr, params: &[(String, Type)]) {
        match expr {
            Expr::Name(name, _) if name != "result" => {
                if let Some(index) = params.iter().position(|(n, _)| n == name) {
                    *name = format!("p{index}");
                }
            }
            Expr::Variant(owner, variant, _) => self.variant(owner, variant),
            Expr::Construct(owner, variant, args, _) => {
                let (e, v) = (owner.clone(), variant.clone());
                self.variant(owner, variant);
                let fields = self.source_enum(&e).fields_of(&v).to_vec();
                for (label, arg) in args {
                    self.expr(arg, params);
                    if let Some(label) = label {
                        *label = field_name(position(&fields, label));
                    }
                }
            }
            Expr::Record(name, fields, _) => {
                let index = self.record_index(name);
                let declared = &self.records[index].fields;
                let mut renamed: Vec<(String, Expr)> = std::mem::take(fields)
                    .into_iter()
                    .map(|(field, value)| (field_name(position(declared, &field)), value))
                    .collect();
                for (_, value) in &mut renamed {
                    self.expr(value, params);
                }
                *fields = renamed;
                *name = format!("R{index}");
            }
            Expr::Field(value, owner, field, _) => {
                self.expr(value, params);
                match owner {
                    Some(Owner::Record(name)) => {
                        let index = self.record_index(name);
                        *field = field_name(position(&self.records[index].fields, field));
                        *name = format!("R{index}");
                    }
                    Some(Owner::Variant(e, variant)) => {
                        let fields = self.source_enum(e).fields_of(variant).to_vec();
                        *field = field_name(position(&fields, field));
                        self.variant(e, variant);
                    }
                    None => {}
                }
            }
            Expr::Is(value, e, variant) => {
                self.expr(value, params);
                self.variant(e, variant);
            }
            Expr::Match(value, arms, _) => {
                self.expr(value, params);
                for arm in arms {
                    if let Some(test) = &mut arm.test {
                        self.expr(test, params);
                    }
                    self.expr(&mut arm.result, params);
                }
            }
            Expr::Not(inner) | Expr::Neg(inner, _) => self.expr(inner, params),
            Expr::Binary(_, left, right) | Expr::Arith(_, left, right, _) => {
                self.expr(left, params);
                self.expr(right, params);
            }
            Expr::If(condition, then, otherwise) => {
                self.expr(condition, params);
                self.expr(then, params);
                self.expr(otherwise, params);
            }
            Expr::List(items, element, _) => {
                for item in items {
                    self.expr(item, params);
                }
                if let Some(t) = element {
                    *t = self.ty(t);
                }
            }
            Expr::Call(name, args, _) => {
                for (label, arg) in args {
                    self.expr(arg, params);
                    *label = None;
                }
                *name = format!("f{}", self.function_index(name));
            }
            // A quantifier's name takes the next position after those in
            // scope, so its normalized name is unique where it is bound. Only
            // names are looked up here, so the type beside it is a stand-in.
            Expr::Quantified(_, name, range, condition, _) => {
                self.expr(range.list_mut(), params);
                let mut inner = params.to_vec();
                inner.push((std::mem::take(name), Type::Bool));
                *name = format!("p{}", params.len());
                self.expr(condition, &inner);
            }
            Expr::Length(list, _) => self.expr(list, params),
            Expr::Next(fields, _) => {
                for (_, value) in fields {
                    self.expr(value, params);
                }
            }
            Expr::Index(list, index, _) => {
                self.expr(list, params);
                self.expr(index, params);
            }
            Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) => {}
        }
    }

    /// The source enum named `name`.
    fn source_enum(&self, name: &str) -> &'a Enum {
        let Some(e) = self.program.enums.iter().find(|e| e.name == name) else {
            unreachable!("`check` rejects the unknown enum `{name}`")
        };
        e
    }

    /// A variant of a source enum, as `E{index}` and `v{position}`.
    fn variant(&mut self, owner: &mut String, variant: &mut String) {
        let index = self.enum_index(owner);
        let position = self.enums[index]
            .variants
            .iter()
            .position(|v| v == variant)
            .unwrap_or_default();
        *owner = format!("E{index}");
        *variant = format!("v{position}");
    }

    /// `source` with its parameters, types and clauses normalized, and its
    /// body as written.
    pub(super) fn contract(&mut self, source: &Function) -> Function {
        let mut function = source.clone();
        function.params = source
            .params
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| (format!("p{index}"), self.ty(ty)))
            .collect();
        function.ret = self.ty(&source.ret);
        for clause in &mut function.clauses {
            self.expr(&mut clause.expr, &source.params);
        }
        function
    }

    /// Normalizes the body of `function`, whose source parameters are `params`.
    pub(super) fn body(&mut self, function: &mut Function, params: &[(String, Type)]) {
        match &mut function.body {
            Body::Expr(body) => self.expr(body, params),
            Body::Decide(rows, _) => {
                for row in rows {
                    if let Some(condition) = &mut row.condition {
                        self.expr(condition, params);
                    }
                    self.expr(&mut row.result, params);
                }
            }
        }
    }
}

/// The canonical text of a normalized function's specification and body,
/// or of its contract alone.
pub(super) fn canonical(function: &Function, with_body: bool) -> String {
    let mut out = String::new();
    for (_, ty) in &function.params {
        out += &format!(" (param {ty:?})");
    }
    out += &format!(" (returns {:?})", function.ret);
    if let Some(member) = function.machine {
        out += &format!(" (member {:?})", member.role);
    }
    for clause in &function.clauses {
        if with_body || clause.kind != ClauseKind::Open {
            out += &format!(
                " ({:?}{} {})",
                clause.kind,
                if clause.helper { " helper" } else { "" },
                crate::obligation::term(&clause.expr)
            );
        }
    }
    if !with_body {
        return out;
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
    out
}
