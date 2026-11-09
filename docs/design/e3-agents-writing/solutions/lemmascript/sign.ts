//@ ensures n > 0 ==> \result === 1
//@ ensures n < 0 ==> \result === -1
//@ ensures n === 0 ==> \result === 0
export function sign(n: number): number {
  if (n > 0) return 1;
  if (n < 0) return -1;
  return 0;
}
