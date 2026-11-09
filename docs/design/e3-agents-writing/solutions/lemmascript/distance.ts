//@ requires a >= -1000000 && a <= 1000000 && b >= -1000000 && b <= 1000000
//@ ensures \result >= 0
//@ ensures \result === a - b || \result === b - a
export function distance(a: number, b: number): number {
  if (a >= b) return a - b;
  return b - a;
}
