//! Normalizing a function: its names replaced by positions, and the
//! canonical text its identity hashes.

use crate::syntax::{Body, ClauseKind, Enum, Expr, Function, Program, Type};

pub(super) struct Renamer<'a> {
    pub(super) program: &'a Program,
    pub(super) enums: Vec<&'a Enum>,
    pub(super) functions: Vec<&'a Function>,
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
            Expr::Index(list, index, _) => {
                self.expr(list, params);
                self.expr(index, params);
            }
            Expr::Bool(_) | Expr::Int(..) | Expr::Text(..) | Expr::Name(..) => {}
        }
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
    for clause in &function.clauses {
        if with_body || clause.kind != ClauseKind::Open {
            out += &format!(
                " ({:?} {})",
                clause.kind,
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
