export type PartialDecision = "deny" | "allow" | "unknown";

//@ ensures \result === "deny" <==> satisfiedForbid || (!satisfiedPermit && !residualPermit)
//@ ensures \result === "allow" <==> !satisfiedForbid && satisfiedPermit && !residualForbid
export function partialDecision(satisfiedForbid: boolean, satisfiedPermit: boolean, residualPermit: boolean,
  residualForbid: boolean): PartialDecision {
  if (satisfiedForbid) return "deny";
  if (!satisfiedPermit && !residualPermit) return "deny";
  if (satisfiedPermit && !residualForbid) return "allow";
  return "unknown";
}
