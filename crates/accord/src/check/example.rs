//! Checks an `example`'s call: the function it names, its labels and the
//! types of its arguments and expected result.

use super::{Scope, type_name};
use crate::Diagnostic;
use crate::syntax::{Enum, Example, Expr, Function};

/// Checks an example's call against its function, and puts its arguments in
/// the order of the parameters, each labelled.
pub(super) fn check(
    enums: &[Enum],
    functions: &[Function],
    example: &mut Example,
) -> Result<(), Diagnostic> {
    let line = example.line;
    let Some(function) = functions.iter().find(|f| f.name == example.function) else {
        return Err(Diagnostic::new(
            line,
            format!("unknown function `{}`", example.function),
        ));
    };
    let count = function.params.len();
    if example.args.len() != count {
        return Err(Diagnostic::new(
            line,
            format!("`{}` takes {count} arguments", function.name),
        ));
    }
    if count > 1 && example.args.iter().any(|(label, _)| label.is_none()) {
        return Err(Diagnostic::new(
            line,
            format!(
                "label every argument: `{}` has more than one parameter",
                function.name
            ),
        ));
    }
    let mut ordered: Vec<Option<Expr>> = vec![None; count];
    for (label, expr) in example.args.drain(..) {
        let index = match &label {
            None => 0,
            Some(label) => function
                .params
                .iter()
                .position(|(name, _)| name == label)
                .ok_or_else(|| {
                    Diagnostic::new(
                        line,
                        format!("`{}` has no parameter `{label}`", function.name),
                    )
                })?,
        };
        if ordered[index].replace(expr).is_some() {
            return Err(Diagnostic::new(
                line,
                format!("`{}` is given twice", function.params[index].0),
            ));
        }
    }
    let scope = Scope {
        enums,
        params: &[],
        result: None,
        spec: false,
    };
    for ((name, ty), expr) in function.params.iter().zip(ordered) {
        let Some(mut expr) = expr else {
            return Err(Diagnostic::new(line, format!("`{name}` is missing")));
        };
        scope.value(
            &mut expr,
            ty,
            &format!("`{name}` needs a `{}`", type_name(ty)),
        )?;
        example.args.push((Some(name.clone()), expr));
    }
    scope.value(
        &mut example.expected,
        &function.ret,
        &format!(
            "`{}` returns a `{}`",
            function.name,
            type_name(&function.ret)
        ),
    )
}
