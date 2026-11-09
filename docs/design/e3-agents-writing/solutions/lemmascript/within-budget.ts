//@ requires first >= 0 && second >= 0 && limit >= 0
//@ ensures \result <==> (first <= limit || second <= limit)
export function withinBudgetAfterRetry(first: number, second: number, limit: number): boolean {
  return first <= limit || second <= limit;
}
