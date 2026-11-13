#!/usr/bin/env bash
# Stage 10's check against a state machine from outside Tesota: opossum's own
# lint and tests, run once as released and once with its breaker state and
# decisions kept in the module `accord build` writes from
# conformance/programs/opossum, through adapter.patch.
#
# Usage: scripts/check-opossum.sh <empty folder> [opossum repository]
# It needs git, Node 22 or later, npm with network access, and the solvers
# (ACCORD_Z3 and ACCORD_CVC5). Both runs' output is kept in the folder.
#
# A few tests are known to fail in both runs on a slow or Windows machine:
# a rolling-window test with a 10 ms window, and health-check timers that
# fire after their test ended. A run passes when lint passes and every failing
# assertion is one of those.
set -euo pipefail

work="${1:?usage: scripts/check-opossum.sh <empty folder> [opossum repository]}"
repository="${2:-https://github.com/nodeshift/opossum}"
commit=decbedf63d7815049233e544dcb351590ff0c84e
root="$(cd "$(dirname "$0")/.." && pwd)"
port="$root/conformance/programs/opossum"
repo="$work/opossum"
known='^not ok [0-9]+ (failures reset because no event emitter is provided and rollingCountTimeout set to 10ms|\.end\(\) already called.*|plan != count)$'

if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
  echo "$work: not empty" >&2
  exit 2
fi
git -c core.autocrlf=false clone --quiet --no-checkout "$repository" "$repo"
git -C "$repo" -c core.autocrlf=false checkout --quiet "$commit"
(cd "$repo" && npm ci --ignore-scripts --no-audit --no-fund > "$work/npm.log" 2>&1)

run() { # label
  local log="$work/$1.log" status=0
  (
    cd "$repo"
    npx eslint --ignore-path .gitignore .
    npx tape test/*.js
  ) > "$log" 2>&1 || status=$?
  echo "$1: $(grep -E '^# (tests|pass|fail)' "$log" | tr '\n' ' ') (output in $log)"
  local others
  others="$(grep -E '^not ok' "$log" | tr -d '\r' | grep -Ev "$known" || true)"
  if ! grep -qE '^# tests ' "$log" || [ -n "$others" ]; then
    echo "$1: failed beyond the known tests (exit $status)" >&2
    [ -n "$others" ] && echo "$others" >&2
    return 1
  fi
}

failed=0
run original || failed=1

(cd "$root" && cargo run --release --quiet -- build "$port" --out "$work/generated")
cp "$work/generated/breaker.ts" "$repo/lib/breaker.mts"
git -C "$repo" apply "$port/adapter.patch"
git -C "$repo" status --short
run generated || failed=1
exit "$failed"
