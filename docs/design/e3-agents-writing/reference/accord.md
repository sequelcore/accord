# Accord reference

Accord is a language whose checker proves each function against its
contract. A file holds type declarations and functions.

## A complete file

```accord
enum Light { Red, Green, Yellow }

/// The light that follows the current one.
fn next_light(current: Light) -> Light
    ensures current == Red implies result == Green
    ensures current == Green implies result == Yellow
    ensures current == Yellow implies result == Red
decide {
    | current == Red   => Green
    | current == Green => Yellow
    | otherwise        => Red
}

/// The bonus for a month: 100 when sales reach the target, otherwise 0.
fn bonus(sales: i64, target: i64) -> i64
    requires sales >= 0 and target >= 0
    ensures sales >= target implies result == 100
    ensures sales < target implies result == 0
{
    if sales >= target { 100 } else { 0 }
}

example next_light(Yellow) == Red
example bonus(sales: 120, target: 100) == 100
```

## Functions

```accord
fn name(param: Type, other: Type) -> Type
    requires condition
    ensures condition
{
    expression
}
```

- `requires condition`: what callers guarantee. It may be repeated.
- `ensures condition`: what the function guarantees about `result`, the value
  it returns. It may be repeated; the checker proves each one.
- The body is a block `{ … }` or a decision table `decide { … }`.
- Clauses go between the signature and the body, `requires` before `ensures`.

## Decision tables

```accord
decide {
    | condition => value
    | otherwise => value
}
```

Rows are tried from the top; the first row whose condition holds gives the
result. `otherwise` matches anything and comes last. The checker proves that
every input reaches a row and that no row is unreachable.

## Types and values

- `Bool`: `true`, `false`
- `i64`, `i32`: signed integers of that width; arithmetic that could overflow is an error the checker reports
- `Text`: `"read"`; compared with `==` and `!=` only
- `List<T>`: `["a", "b"]`, `[]`, for `T` a `Bool`, enum or `Text`; `x in list` tests membership
- `enum Name { A, B }`: variants written bare, `A`, where the type is known

## Expressions

- Logic: `and`, `or`, `not`; in `requires` and `ensures` also `implies`
- Comparison: `==`, `!=`; `<`, `<=`, `>`, `>=` on integers; `x in list`
- Arithmetic: `+`, `-`, `*`, and `-` before a value; no division
- Choice: `if condition { a } else { b }`, always with `else`
- Call: `name(value)` with one parameter, `name(a: 1, b: 2)` with more

A block's value is its last expression; there is no `return` and no loop. An
early exit is a decision table row. In `requires` and `ensures`, integers are
mathematical and never overflow. There is no `<==>`: write an equivalence as
two `implies`.

## Examples

`example call == value` states a concrete case; the checker runs it. Examples
are optional.

## Checking

The checker prints, for each function, its specification, each obligation
`proved` or `refuted` with a counterexample (`fails with n = 3`), and any
**open case**: an input for which the contract allows two results. An open
case means the contract doesn't fully state what the function returns. A
syntax error is printed as `file:line: message`.

Comments start with `//`; `///` documents the declaration below it.
