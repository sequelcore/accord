#!/usr/bin/env bash
# Runs every case in cases/ five times with Z3 and cvc5, with the options
# Accord uses, and prints each case's answers and its minimum, median and
# maximum time in milliseconds. Each case asserts the negation of a claim:
# `unsat` proves it, `sat` refutes it. The cases are written by gen.py.
#
# Usage: ACCORD_Z3=... ACCORD_CVC5=... [CASES=cases] [Z3_OPTS="smt.arith.solver=2"]
#        [ONLY=z3|cvc5] docs/design/measurements/2026-11-13-semver-contract/run.sh [pattern]
#   CASES: the folder of cases, relative to this script (default cases).
#   Z3_OPTS: more options for Z3, after Accord's own.
#   ONLY: run one solver, and leave the other's column as "-".
set -euo pipefail
cd "$(dirname "$0")/${CASES:-cases}"
RUNS=${RUNS:-5}
Z3_OPTS=${Z3_OPTS:-}
ONLY=${ONLY:-}
now() { date +%s%N; }
solve() { # solver cases... -> "answers min med max"
  local f=$1 shift_cmd=("${@:2}") answers=() times=() s o
  for _ in $(seq $RUNS); do
    s=$(now)
    o=$("${shift_cmd[@]}" "$f" 2>&1 | head -n 1 | tr -d '\r' | sed 's/^cvc5 interrupted by timeout\.$/timeout/') || true
    times+=($(( ($(now) - s) / 1000000 )))
    answers+=("$o")
  done
  local sorted
  sorted=($(printf '%s\n' "${times[@]}" | sort -n))
  local distinct
  distinct=$(printf '%s\n' "${answers[@]}" | sort | uniq -c | awk '{printf "%s%s", (NR>1?"/":""), $2 (($1<'$RUNS')? "x" $1 : "")}')
  echo "$distinct ${sorted[0]} ${sorted[$((RUNS / 2))]} ${sorted[$((RUNS - 1))]}"
}
printf '%-46s %-26s %-26s\n' case "z3 (answer min med max ms)" "cvc5 --mbqi (answer min med max ms)"
for f in ${1:-*.smt2}; do
  z=-; c=-
  # shellcheck disable=SC2086 # Z3_OPTS is a list of options
  [ "$ONLY" = cvc5 ] || z=$(solve "$f" "$ACCORD_Z3" -smt2 -t:10000 $Z3_OPTS)
  [ "$ONLY" = z3 ] || c=$(solve "$f" "$ACCORD_CVC5" --lang smt2 --mbqi --tlimit-per=10000)
  printf '%-46s %-26s %-26s\n' "${f%.smt2}" "$z" "$c"
done
