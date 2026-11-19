#!/usr/bin/env bash
# Saves each script Accord sends to Z3 in $CAPTURE, then runs Z3 on it.
# Use it as ACCORD_Z3, with the real Z3 in REAL_Z3, to capture a function's
# scripts: one file per obligation, named by time.
set -euo pipefail
file="$CAPTURE/$(date +%s%N)-$$.smt2"
cat > "$file"
exec "$REAL_Z3" "$@" < "$file"
