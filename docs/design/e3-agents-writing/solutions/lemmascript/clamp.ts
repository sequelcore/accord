//@ requires low <= high
//@ ensures low <= \result && \result <= high
//@ ensures low <= value && value <= high ==> \result === value
//@ ensures value < low ==> \result === low
//@ ensures value > high ==> \result === high
export function clamp(value: number, low: number, high: number): number {
  if (value < low) return low;
  if (value > high) return high;
  return value;
}
