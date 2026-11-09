# LemmaScript reference

LemmaScript proves TypeScript functions against contracts written in `//@`
comments. The checker translates the file to Dafny and proves it there. A
file holds type declarations and exported functions.

## A complete file

```ts
export type Light = "red" | "green" | "yellow";

/** The light that follows the current one. */
//@ ensures current === "red" ==> \result === "green"
//@ ensures current === "green" ==> \result === "yellow"
//@ ensures current === "yellow" ==> \result === "red"
export function nextLight(current: Light): Light {
  if (current === "red") return "green";
  if (current === "green") return "yellow";
  return "red";
}

/** The bonus for a month: 100 when sales reach the target, otherwise 0. */
//@ requires sales >= 0 && target >= 0
//@ ensures sales >= target ==> \result === 100
//@ ensures sales < target ==> \result === 0
export function bonus(sales: number, target: number): number {
  return sales >= target ? 100 : 0;
}
```

## Functions

```ts
//@ requires condition
//@ ensures condition
export function name(param: Type, other: Type): Type {
  statements
}
```

- `//@ requires condition`: what callers guarantee. It may be repeated.
- `//@ ensures condition`: what the function guarantees about `\result`, the
  value it returns. It may be repeated; the checker proves each one.
- The `//@` lines go directly above the function they belong to.

## Loops

```ts
let k = 0;
while (k < items.length) {
  //@ invariant 0 <= k && k <= items.length
  //@ invariant forall(j: nat, j < k ==> items[j] !== target)
  if (items[k] === target) return true;
  k = k + 1;
}
```

A loop needs `//@ invariant` lines, first inside its body, stating what holds
before each iteration; the checker proves them. `//@ decreases expression`
can state why the loop ends when the checker can't tell.

## Types and values

- `boolean`: `true`, `false`
- `number`: integers; the checker treats them as mathematical integers
- `string`: `"read"`; compared with `===` and `!==`
- `readonly T[]`: `["a", "b"]`, `[]`; `.length` and indexing `items[k]`
- `type Name = "a" | "b"`: string literal unions, compared with `===`

## Expressions

- Logic: `&&`, `||`, `!`; in `//@` lines also `==>` (implies) and `<==>` (if and only if)
- Comparison: `===`, `!==`; `<`, `<=`, `>`, `>=` on numbers
- Arithmetic: `+`, `-`, `*`, and `-` before a value
- Choice: `if`/`else` statements, `return`, and `condition ? a : b`
- Quantifiers: in `//@` lines: `forall(k: nat, condition)`, `exists(k: nat, condition)`

## Checking

The checker prints `Dafny program verifier finished with N verified, M
errors`. Each error names a position in the generated `.dfy` file, which sits
beside your file and may be read, with the clause that could not be proved:
`a postcondition could not be proved on this return path`, followed by the
`ensures` it refers to. A translation error names the TypeScript it couldn't
handle.

Comments start with `//`; `/** … */` documents the declaration below it.
