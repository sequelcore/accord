"""Writes one variant of a captured script.

Usage: python3 variants.py <none|A|B|saturate> <in.smt2> <out.smt2>

- none: the script as Accord at 55772e0 wrote it.
- A: a length limit on every String constant and every String field of a
  record constant, as a limit on parameters and results would add.
- B: a limit at every `(str.len X)` an assertion holds, under the guards and
  `let`s of the quantifiers around it, after expanding `define-fun` uses.
- saturate: every `(str.len X)` replaced by `(ite (<= (str.len X) L) (str.len X) L)`.
"""
import re
import sys

LIMIT = "9007199254740991"


def parse(text):
    tokens, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c in " \t\r\n":
            i += 1
        elif c == ";":
            while i < n and text[i] != "\n":
                i += 1
        elif c in "()":
            tokens.append(c)
            i += 1
        elif c == '"':
            j = i + 1
            while True:
                if text[j] == '"':
                    if j + 1 < n and text[j + 1] == '"':
                        j += 2
                        continue
                    break
                j += 1
            tokens.append(text[i : j + 1])
            i = j + 1
        elif c == "|":
            j = text.index("|", i + 1)
            tokens.append(text[i : j + 1])
            i = j + 1
        else:
            j = i
            while j < n and text[j] not in " \t\r\n()":
                j += 1
            tokens.append(text[i:j])
            i = j
    stack = [[]]
    for t in tokens:
        if t == "(":
            stack.append([])
        elif t == ")":
            done = stack.pop()
            stack[-1].append(done)
        else:
            stack[-1].append(t)
    return stack[0]


def show(e):
    return e if isinstance(e, str) else "(" + " ".join(show(x) for x in e) + ")"


def subst(e, env):
    if isinstance(e, str):
        return env.get(e, e)
    if e and e[0] in ("forall", "exists"):
        names = {b[0] for b in e[1]}
        inner = {k: v for k, v in env.items() if k not in names}
        return [e[0], e[1]] + [subst(x, inner) for x in e[2:]]
    if e and e[0] == "let":
        binds = [[b[0], subst(b[1], env)] for b in e[1]]
        names = {b[0] for b in e[1]}
        inner = {k: v for k, v in env.items() if k not in names}
        return ["let", binds, subst(e[2], inner)]
    return [subst(x, env) for x in e]


def expand(e, defs):
    if isinstance(e, str):
        if e in defs and not defs[e][0]:
            return expand(defs[e][1], defs)
        return e
    e = [expand(x, defs) for x in e]
    if e and isinstance(e[0], str) and e[0] in defs and defs[e[0]][0]:
        params, body = defs[e[0]]
        return expand(subst(body, dict(zip(params, e[1:]))), defs)
    return e


def sites(e, frames, found):
    if isinstance(e, str):
        return
    if e and e[0] == "str.len":
        found.append((e, list(frames)))
    if e and e[0] in ("forall", "exists"):
        body = e[2]
        guard = "true"
        if isinstance(body, list) and body and body[0] in ("=>", "and") and len(body) == 3:
            guard = body[1]
        frames.append(("q", e[1], guard))
        sites(body, frames, found)
        frames.pop()
        return
    if e and e[0] == "let":
        for b in e[1]:
            sites(b[1], frames, found)
        frames.append(("let", e[1]))
        sites(e[2], frames, found)
        frames.pop()
        return
    for x in e:
        sites(x, frames, found)


def axiom(site, frames):
    fact = ["<=", site, LIMIT]
    for frame in reversed(frames):
        if frame[0] == "let":
            fact = ["let", frame[1], fact]
        else:
            fact = ["forall", frame[1], ["=>", frame[2], fact]]
    return fact


def limit_b(text):
    commands = parse(text)
    defs, facts, out = {}, [], []
    for c in commands:
        if c[0] == "define-fun":
            defs[c[1]] = ([p[0] for p in c[2]], c[4])
        if c[0] == "assert":
            found = []
            sites(expand(c[1], defs), [], found)
            for site, frames in found:
                fact = show(axiom(site, frames))
                if fact not in facts:
                    facts.append(fact)
    for c in commands:
        if c[0] == "check-sat":
            out += ["(assert " + f + ")" for f in facts]
        out.append(show(c))
    return "\n".join(out) + "\n"


def limit_a(text):
    lines = text.splitlines()
    fields = {}
    for line in lines:
        if line.startswith("(declare-datatypes"):
            for sort, body in re.findall(r"\(\((\w+) \d+\)\) \(\((.*?)\)\)\)", line):
                fields[sort] = re.findall(r"\(([\w.]+) String\)", body)
    out = []
    for line in lines:
        out.append(line)
        m = re.match(r"\(declare-const (\S+) (\S+)\)$", line)
        if m:
            name, sort = m.groups()
            if sort == "String":
                out.append(f"(assert (<= (str.len {name}) {LIMIT}))")
            for selector in fields.get(sort, []):
                out.append(f"(assert (<= (str.len ({selector} {name})) {LIMIT}))")
    return "\n".join(out) + "\n"


def saturate(e):
    if isinstance(e, str):
        return e
    e = [saturate(x) for x in e]
    if e and e[0] == "str.len":
        return ["ite", ["<=", e, LIMIT], e, LIMIT]
    return e


def main():
    mode, src, dst = sys.argv[1:4]
    text = open(src, encoding="utf-8").read()
    if mode == "none":
        out = text
    elif mode == "A":
        out = limit_a(text)
    elif mode == "B":
        out = limit_b(text)
    elif mode == "saturate":
        out = "\n".join(show(saturate(c)) for c in parse(text)) + "\n"
    else:
        sys.exit(f"unknown variant {mode}")
    open(dst, "w", encoding="utf-8", newline="\n").write(out)


main()
