#!/usr/bin/env bash
# Writes the none, A, B and saturate variants of every script in cases/ and
# runs each with Z3 and cvc5, with Accord's flags and a 10 s limit, under
# seeds 1 to 5. Prints one line per run: case, variant, seed, each solver's
# first line of output and its time in milliseconds.
#
# Usage: ACCORD_Z3=... ACCORD_CVC5=... docs/design/measurements/2026-11-18-text-length-limit/run.sh
set -uo pipefail
cd "$(dirname "$0")"
PYTHON=${PYTHON:-python3}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
first() {
  "$@" 2>&1 | head -n 1 | sed -E 's/^cvc5 interrupted by timeout.$/timeout/; s/^The model was computed.*/model-too-long/'
}
ms() { echo $(( ($(date +%s%N) - $1) / 1000000 )); }
for f in cases/*/*.smt2; do
  id=${f#cases/}; id=${id%.smt2}
  for variant in none A B saturate; do
    out="$work/${id//\//-}-$variant.smt2"
    "$PYTHON" variants.py "$variant" "$f" "$out"
    for seed in 1 2 3 4 5; do
      start=$(date +%s%N)
      z=$(first timeout 40 "$ACCORD_Z3" -smt2 -t:10000 smt.random_seed=$seed sat.random_seed=$seed "$out")
      zt=$(ms "$start"); start=$(date +%s%N)
      c=$(first timeout 40 "$ACCORD_CVC5" --lang smt2 --mbqi --tlimit-per=10000 --seed=$seed "$out")
      ct=$(ms "$start")
      printf '%s %s %s | %s %s | %s %s\n' "$id" "$variant" "$seed" "${z:-crash}" "$zt" "${c:-crash}" "$ct"
    done
  done
done
