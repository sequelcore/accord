export type Grade = "a" | "b" | "c" | "f";

//@ requires score >= 0 && score <= 100
//@ ensures \result === "a" <==> score >= 90
//@ ensures \result === "b" <==> score >= 80 && score < 90
//@ ensures \result === "c" <==> score >= 70 && score < 80
//@ ensures \result === "f" <==> score < 70
export function grade(score: number): Grade {
  if (score >= 90) return "a";
  if (score >= 80) return "b";
  if (score >= 70) return "c";
  return "f";
}
