#!/usr/bin/env bash
# Runs opossum's lint and tests as released, then with its state flags and
# decisions replaced by machine.ts (a hand-written stand-in for a generated
# module) through adapter.patch, then with three faults planted in the
# stand-in. It also runs the two behavior checks against the release.
#
# Usage: docs/design/measurements/2026-11-12-stage-10-machines/opossum/run.sh <empty folder>
# It needs git, Node 22 or later, and npm with network access. Each run's
# output is kept in the folder.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
work="${1:?usage: run.sh <empty folder>}"
commit=decbedf63d7815049233e544dcb351590ff0c84e
repo="$work/opossum"

git -c core.autocrlf=false clone --quiet https://github.com/nodeshift/opossum "$repo"
git -C "$repo" -c core.autocrlf=false checkout --quiet "$commit"
(cd "$repo" && npm ci --ignore-scripts --no-audit --no-fund > "$work/npm.log" 2>&1)

echo "== behavior of the release"
node "$here/invariants.cjs" "$repo"
node "$here/stale-success.cjs" "$repo"

run() { # label
  local log="$work/$1.log"
  (cd "$repo" && npx eslint --ignore-path .gitignore . && npx tape test/*.js) > "$log" 2>&1 || true
  echo "$1: $(grep -E '^# (tests|fail)' "$log" | tr '\n' ' ')"
  grep '^not ok' "$log" | cut -c1-100 || true
}

run original
cp "$here/machine.ts" "$repo/lib/machine.mts"
git -C "$repo" apply "$here/adapter.patch"
run stand-in

plant() { # label old new
  cp "$here/machine.ts" "$repo/lib/machine.mts"
  python - "$repo/lib/machine.mts" "$2" "$3" <<'EOF'
import sys
path, old, new = sys.argv[1:]
text = open(path).read()
assert text.count(old) == 1, old
open(path, "w").write(text.replace(old, new))
EOF
  run "$1"
}
plant fault-no-probe 'return s.phase === "closed" || s.pendingClose;' 'return s.phase === "closed";'
plant fault-at-threshold 'return x * 100n > t * f || halfOpen;' 'return x * 100n >= t * f || halfOpen;'
plant fault-every-call 'return { ...checkBreaker(b), pendingClose: false };' 'return checkBreaker(b);'
