#!/usr/bin/env bash
# Runs every case in cases/ with Z3, cvc5, and cvc5 with --mbqi, 10 s each,
# and prints the first line of each answer. Each case asserts the negation of
# a claim: unsat proves it, sat refutes it.
#
# Usage: ACCORD_Z3=... ACCORD_CVC5=... docs/design/measurements/2026-11-12-list-encoding/run.sh
set -euo pipefail
cd "$(dirname "$0")/cases"
first() { "$@" 2>&1 | head -n 1 | sed "s/^cvc5 interrupted by timeout.$/timeout/" || true; }
printf '%-38s %-8s %-8s %s\n' case z3 cvc5 cvc5-mbqi
for f in *.smt2; do
  z=$(first "$ACCORD_Z3" -T:10 "$f")
  c=$(first "$ACCORD_CVC5" --tlimit=10000 --produce-models "$f")
  m=$(first "$ACCORD_CVC5" --tlimit=10000 --produce-models --mbqi "$f")
  printf '%-38s %-8s %-8s %s\n' "${f%.smt2}" "$z" "$c" "$m"
done
