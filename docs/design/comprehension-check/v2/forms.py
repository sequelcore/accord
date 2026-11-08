"""Write the three forms of every snippet from one template.

Usage: python forms.py

In templates/, a variant is written `{Type.Variant}`. Each form spells it one
way, and nothing else differs between forms:

  short      Variant         bare, as where the type is known
  qualified  Type.Variant    always qualified
  dot        .Variant        a leading dot where the type is known

Arrows in decision tables are realigned after spelling, as the formatter
would.
"""

import pathlib
import re

HERE = pathlib.Path(__file__).parent
SPELLINGS = {
    "short": lambda kind, name: name,
    "qualified": lambda kind, name: f"{kind}.{name}",
    "dot": lambda kind, name: f".{name}",
}
VARIANT = re.compile(r"\{([A-Z]\w*)\.([A-Z]\w*)\}")


def align(text: str) -> str:
    lines = text.split("\n")
    out, block = [], []
    for line in lines + [""]:
        if line.lstrip().startswith("|") and "=>" in line:
            block.append(line)
            continue
        if block:
            width = max(len(row.split("=>")[0].rstrip()) for row in block)
            out += [f"{row.split('=>')[0].rstrip():<{width}} => {row.split('=>', 1)[1].strip()}" for row in block]
            block = []
        out.append(line)
    return "\n".join(out[:-1])


def main() -> None:
    for template in sorted((HERE / "templates").glob("*.txt")):
        source = template.read_text(encoding="utf-8")
        for form, spell in SPELLINGS.items():
            text = align(VARIANT.sub(lambda m: spell(m.group(1), m.group(2)), source))
            target = HERE / "forms" / form / template.name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
