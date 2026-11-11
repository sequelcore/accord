#!/usr/bin/env bash
# Stage 8's check against a rule from outside Tesota: axios-retry's own lint
# and specs, run once as released and once with its request predicates
# replaced by the module `accord build` writes from
# conformance/programs/axios-retry, through adapter.patch.
#
# Usage: scripts/check-axios-retry.sh <empty folder> [axios-retry repository]
# It needs git, npm with network access, the solvers (ACCORD_Z3 and
# ACCORD_CVC5) and `npx node@20`: axios-retry's CI runs Node 16 to 20, and
# its specs don't load under later versions. Both runs' output is kept in
# the folder.
#
# A few specs assert an upper bound on elapsed time ("should take more than
# … milliseconds") and fail now and then on a slow machine, in both runs. A
# run passes when lint passes and every failing spec is one of those.
set -euo pipefail

work="${1:?usage: scripts/check-axios-retry.sh <empty folder> [axios-retry repository]}"
repository="${2:-https://github.com/softonic/axios-retry}"
commit=a748627fdc5e22e659055b6cfb44e35ad5801bf4
root="$(cd "$(dirname "$0")/.." && pwd)"
port="$root/conformance/programs/axios-retry"

if [ -e "$work" ] && [ -n "$(ls -A "$work")" ]; then
  echo "$work: not empty" >&2
  exit 2
fi
git -c core.autocrlf=false clone --quiet --no-checkout "$repository" "$work/axios-retry"
git -C "$work/axios-retry" -c core.autocrlf=false checkout --quiet "$commit"
(cd "$work/axios-retry" && npm ci --ignore-scripts --no-audit --no-fund)

run() { # label
  local log="$work/$1.log" status=0
  (
    cd "$work/axios-retry"
    npx eslint .
    npx -y node@20 node_modules/ts-node/dist/bin.js --project tsconfig-cjs.json node_modules/jasmine/bin/jasmine
  ) > "$log" 2>&1 || status=$?
  local summary others
  summary="$(grep -E '^[0-9]+ specs?, ' "$log" || echo 'no specs ran')"
  others="$(grep -E '^[0-9]+\) ' "$log" | grep -v 'milliseconds' || true)"
  echo "$1: $summary (output in $log)"
  if ! grep -qE '^[0-9]+ specs?, ' "$log" || [ -n "$others" ]; then
    echo "$1: failed beyond the timing specs (exit $status)" >&2
    [ -n "$others" ] && echo "$others" >&2
    return 1
  fi
}

run original || echo "The original already fails: compare the two logs." >&2

(cd "$root" && cargo run --release --quiet -- build "$port" --out "$work/generated")
cp "$work/generated/retry.ts" "$work/axios-retry/src/retry.ts"
git -C "$work/axios-retry" apply "$port/adapter.patch"
git -C "$work/axios-retry" status --short
run generated
