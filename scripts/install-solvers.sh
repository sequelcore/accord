#!/usr/bin/env bash
# Downloads the pinned Z3 and cvc5 releases for this platform into a folder,
# verifies each archive against its pinned SHA-256, and prints the
# ACCORD_Z3 and ACCORD_CVC5 values to use. Under GitHub Actions it also
# writes them to $GITHUB_ENV.
#
# Usage: scripts/install-solvers.sh <folder>
set -euo pipefail

dir="${1:?usage: scripts/install-solvers.sh <folder>}"
z3_tag="z3-5.3.3"
cvc5_tag="cvc5-1.4.2"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    z3_asset="z3-5.3.3-x64-glibc-2.39.zip"; z3_sum="d56f60b06f432f33de5810f7f1a384a339131f361e2d54d08ea83eea14326880"
    cvc5_asset="cvc5-Linux-x86_64-static.zip"; cvc5_sum="7eb18f8c814c36a46c5a7d003983e9cfbc4b0a9828d6dd596239291bd4cc61d8"
    exe="" ;;
  Darwin-arm64)
    z3_asset="z3-5.3.3-arm64-osx-13.3.zip"; z3_sum="6957e6c75266d7ca8fa77030e86e83d78624567dcdf34cc11f4192fdc6f93c6b"
    cvc5_asset="cvc5-macOS-arm64-static.zip"; cvc5_sum="24397f7d022e755876ca836193d4c5f9c1d4326f0f6c01f7d7fc1ddd6bc5096c"
    exe="" ;;
  MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64)
    z3_asset="z3-5.3.3-x64-win.zip"; z3_sum="4e48624a6895b49606b3cf91ca4fbcc069fb2973b9706532170a144baa3fd798"
    cvc5_asset="cvc5-Win64-x86_64-static.zip"; cvc5_sum="9055de9b724724890ba5a19cfdc98f68a8c0653284afa843096dec432a0f88c3"
    exe=".exe" ;;
  *) echo "no pinned solvers for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

# Hashes from stdin: with a file name, sha256sum on Windows escapes the name
# and prefixes the hash with a backslash.
sha256() { if command -v sha256sum >/dev/null; then sha256sum < "$1"; else shasum -a 256 < "$1"; fi | cut -d' ' -f1; }

fetch() { # repo tag asset sum target
  mkdir -p "$5"
  curl -fsSL -o "$5/$3" "https://github.com/$1/releases/download/$2/$3"
  local got; got="$(sha256 "$5/$3")"
  if [ "$got" != "$4" ]; then echo "$3: checksum mismatch ($got)" >&2; exit 1; fi
  unzip -q -o "$5/$3" -d "$5"
}

fetch Z3Prover/z3 "$z3_tag" "$z3_asset" "$z3_sum" "$dir/$z3_tag"
fetch cvc5/cvc5 "$cvc5_tag" "$cvc5_asset" "$cvc5_sum" "$dir/$cvc5_tag"

z3="$(find "$dir/$z3_tag" -type f -name "z3$exe" -path '*/bin/*' | head -n 1)"
cvc5="$(find "$dir/$cvc5_tag" -type f -name "cvc5$exe" -path '*/bin/*' | head -n 1)"
chmod +x "$z3" "$cvc5"
"$z3" --version
"$cvc5" --version | head -n 1

echo "ACCORD_Z3=$z3"
echo "ACCORD_CVC5=$cvc5"
if [ -n "${GITHUB_ENV:-}" ]; then
  { echo "ACCORD_Z3=$z3"; echo "ACCORD_CVC5=$cvc5"; } >> "$GITHUB_ENV"
fi
