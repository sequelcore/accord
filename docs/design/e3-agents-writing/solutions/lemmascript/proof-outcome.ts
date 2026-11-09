export type ProofOutcome = "passed" | "failed" | "vacuous" | "timed_out" | "cancelled" | "not_started";

//@ requires verified >= 0
//@ ensures cancelled ==> \result === "cancelled"
//@ ensures !cancelled && timedOut ==> \result === "timed_out"
//@ ensures !cancelled && !timedOut && !started ==> \result === "not_started"
//@ ensures !cancelled && !timedOut && started && (!regenerated || checkExit !== 0) ==> \result === "failed"
//@ ensures !cancelled && !timedOut && started && regenerated && checkExit === 0 && verified > 0 ==> \result === "passed"
//@ ensures !cancelled && !timedOut && started && regenerated && checkExit === 0 && verified === 0 ==> \result === "vacuous"
export function proofOutcome(cancelled: boolean, timedOut: boolean, started: boolean, regenerated: boolean,
  checkExit: number, verified: number): ProofOutcome {
  if (cancelled) return "cancelled";
  if (timedOut) return "timed_out";
  if (!started) return "not_started";
  if (!regenerated || checkExit !== 0) return "failed";
  return verified > 0 ? "passed" : "vacuous";
}
