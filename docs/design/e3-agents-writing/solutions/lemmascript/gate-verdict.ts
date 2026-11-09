export type ProofOutcome = "passed" | "failed" | "vacuous" | "timed_out" | "cancelled" | "not_started";
export type GateVerdict = "proved" | "send_back" | "no_progress" | "operator";

//@ ensures \result === "proved" <==> outcome === "passed"
//@ ensures \result === "send_back" <==> (outcome === "failed" || outcome === "vacuous") && !exists(k: nat, k < sent.length && sent[k] === failure)
//@ ensures \result === "no_progress" <==> (outcome === "failed" || outcome === "vacuous") && exists(k: nat, k < sent.length && sent[k] === failure)
//@ ensures \result === "operator" <==> (outcome === "not_started" || outcome === "timed_out" || outcome === "cancelled")
export function gateVerdict(outcome: ProofOutcome, failure: string, sent: readonly string[]): GateVerdict {
  if (outcome === "passed") return "proved";
  if (outcome !== "failed" && outcome !== "vacuous") return "operator";
  let k = 0;
  while (k < sent.length) {
    //@ invariant 0 <= k && k <= sent.length
    //@ invariant forall(j: nat, j < k ==> sent[j] !== failure)
    if (sent[k] === failure) return "no_progress";
    k = k + 1;
  }
  return "send_back";
}
