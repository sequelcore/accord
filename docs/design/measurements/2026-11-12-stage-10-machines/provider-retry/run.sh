#!/usr/bin/env bash
# Writes ProviderRetry's inductive obligations with each set of helper
# invariants and each planted fault, runs Z3 and cvc5 on every file with the
# options Accord uses (10 s each, cvc5 with --mbqi), then explores the
# machine's reachable states with and without each fault.
#
# Usage: ACCORD_Z3=... ACCORD_CVC5=... docs/design/measurements/2026-11-12-stage-10-machines/provider-retry/run.sh <empty folder>
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
work="${1:?usage: run.sh <empty folder>}"
first() { "$@" 2>&1 | head -n 1 | sed "s/^cvc5 interrupted by timeout.$/timeout/" || true; }
for variant in "none" "three" "four" "four IgnoreCancel" "four ReplayTool"; do
  out="$work/${variant// /-}"
  python "$here/gen.py" "$out" $variant
  echo "== helpers and fault: $variant"
  for f in "$out"/*.smt2; do
    z=$(first "$ACCORD_Z3" -smt2 -t:10000 "$f")
    c=$(first "$ACCORD_CVC5" --lang smt2 --mbqi --tlimit-per=10000 "$f")
    printf '  %-20s z3=%-8s cvc5=%s\n' "$(basename "$f" .smt2)" "$z" "$c"
  done
done
for fault in none IgnoreCancel ReplayTool; do python "$here/explore.py" "$fault"; done
