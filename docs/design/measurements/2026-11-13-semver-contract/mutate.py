"""Mutators for the stability runs: they rewrite an SMT-LIB case without
changing what it says, as Mariposa does (https://github.com/secure-foundations/mariposa).

shuffle(text, seed): moves every declaration before the first assertion, then
  puts the assertions in a random order.
rename(text, seed): gives every symbol the script declares or binds a random
  name; datatype names, constructors, selectors and the theories' symbols stay.
"""
import random
import re

from gen import balanced

TOKEN = re.compile(r'"(?:[^"]|"")*"|[^\s()"]+')
BINDER = re.compile(r"\((\w[\w!.]*) (Int|PreId|Version|Fuel|Bool|String|\(Seq PreId\))\)")
DECLARED = re.compile(r"\((?:define-fun|define-fun-rec|declare-const|declare-fun|declare-sort) ([^\s()]+)")


KEEP = {"Seq", "digits", "text", "major", "minor", "patch", "pre", "build", "Numeric", "Alpha", "Ver"}


def strip_comments(text):
    return "".join(l for l in text.splitlines(True) if not l.lstrip().startswith(";"))


def commands(text):
    """The top-level commands."""
    text = strip_comments(text)
    out, i = [], 0
    while True:
        j = text.find("(", i)
        if j < 0:
            return out
        end = balanced(text, j)
        out.append(text[i:end].strip("\n"))
        i = end


def shuffle(text, seed):
    rng = random.Random(seed)
    cmds = commands(text)
    asserts = [c for c in cmds if c.lstrip("; \n").startswith("(assert ") or c.startswith("(assert ")]
    rest = [c for c in cmds if c not in asserts and not c.rstrip().endswith("(check-sat)")]
    check = [c for c in cmds if c.rstrip().endswith("(check-sat)")]
    rng.shuffle(asserts)
    # Declarations first, in their order; then the assertions; then the check.
    return "\n".join(rest + asserts + check) + "\n"


def rename(text, seed):
    rng = random.Random(seed)
    names = (set(DECLARED.findall(text)) | {m[1] for m in BINDER.finditer(text)}) - KEEP
    text = strip_comments(text)
    mapping = {n: "v" + "".join(rng.choice("abcdefghijklmnopqrstuvwxyz0123456789") for _ in range(10)) for n in sorted(names)}
    return TOKEN.sub(lambda m: mapping.get(m[0], m[0]), text)
