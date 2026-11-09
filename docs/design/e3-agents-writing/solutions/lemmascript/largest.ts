//@ ensures \result >= a && \result >= b && \result >= c
//@ ensures \result === a || \result === b || \result === c
export function largest(a: number, b: number, c: number): number {
  if (a >= b && a >= c) return a;
  if (b >= c) return b;
  return c;
}
