#!/usr/bin/env bash
# Stage 8's done check: Tesota's own typecheck and tests, run once with its
# proved rules as written and once with the modules `accord build` writes
# from conformance/programs/tesota in their place.
#
# Usage: scripts/check-tesota.sh <empty folder> [Tesota repository]
# The folder receives a clone of Tesota at the commit the ports come from.
# It needs git, bun, the solvers (ACCORD_Z3 and ACCORD_CVC5) and network
# access for `bun install`. Both runs' output is kept in the folder.
set -euo pipefail

work="${1:?usage: scripts/check-tesota.sh <empty folder> [Tesota repository]}"
repository="${2:-https://github.com/sequelcore/tesota}"
commit=4ef9b191b24204d633af72c03f1c398268364573
rules=(proof-outcome-rule gate-rule mutation-rule)
root="$(cd "$(dirname "$0")/.." && pwd)"

if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
  echo "$work: not empty" >&2
  exit 2
fi
git clone --quiet --no-checkout "$repository" "$work/tesota"
git -C "$work/tesota" checkout --quiet "$commit"
(cd "$work/tesota" && bun install --frozen-lockfile)

run() { # label
  local status=0
  (cd "$work/tesota" && bun run typecheck && bun run test) > "$work/$1.log" 2>&1 || status=$?
  echo "$1: exit $status (output in $work/$1.log)"
  return "$status"
}

run original || echo "The original already fails: compare the two logs." >&2

(cd "$root" && cargo run --release --quiet -- build conformance/programs/tesota --out "$work/generated")
for rule in "${rules[@]}"; do
  cp "$work/generated/$rule.ts" "$work/tesota/src/verification/$rule.ts"
done
git -C "$work/tesota" status --short
run generated
