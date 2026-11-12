//! Checks an `example`'s call: the function it names, its labels and the
//! types of its arguments and expected result.

use super::{Scope, ordered, type_name};
use crate::Diagnostic;
use crate::syntax::{Enum, Example, Function};

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
    let args = std::mem::take(&mut example.args);
    let ordered = ordered(&function.name, &function.params, args, line)?;
    let scope = Scope {
        enums,
        params: &[],
        result: None,
        spec: false,
        functions: None,
    };
    for ((name, ty), mut expr) in function.params.iter().zip(ordered) {
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
