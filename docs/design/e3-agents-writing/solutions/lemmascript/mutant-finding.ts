export type ProofOutcome = "passed" | "failed" | "vacuous" | "timed_out" | "cancelled" | "not_started";
export type MutantFinding = "rejected" | "survived" | "inconclusive";

//@ ensures \result === "rejected" <==> outcome === "failed"
//@ ensures \result === "survived" <==> outcome === "passed"
//@ ensures \result === "inconclusive" <==> outcome !== "failed" && outcome !== "passed"
export function mutantFinding(outcome: ProofOutcome): MutantFinding {
  if (outcome === "failed") return "rejected";
  if (outcome === "passed") return "survived";
  return "inconclusive";
}
