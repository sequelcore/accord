"""Score replies.jsonl against the answer key in README.md.

Usage: python score.py

Scoring is by pattern: question 2's first yes or no is the verdict, and a
detection needs question 3's input to have timed_out true and the reply to say
it returns Passed. Replies the patterns can't read are listed for reading by
hand.
"""

import collections
import json
import pathlib
import re

HERE = pathlib.Path(__file__).parent


def section(text: str, number: int) -> str:
    match = re.search(rf"(?:^|\s){number}[.):]\s*(.*?)(?:\s{number + 1}[.):]\s|$)", text)
    return match.group(1) if match else ""


def score(record: dict) -> str:
    text = re.sub(r"[*`#]", "", re.sub(r"\s+", " ", record["reply"]))
    verdict = re.search(r"\b(yes|no)\b", section(text, 2), re.IGNORECASE)
    verdict = verdict.group(1).lower() if verdict else "unread"
    witness = section(text, 3)
    timed_out = re.search(r"timed_out\s*[:=]\s*true|\(false, true, true, true, 0", witness)
    if record["variant"].endswith("1"):
        if verdict == "no" and timed_out and "Passed" in witness:
            return "detected"
        return "no, wrong input" if verdict == "no" else verdict if verdict == "unread" else "missed"
    return {"yes": "correct", "no": "false alarm"}.get(verdict, "unread")


def main() -> None:
    records = [json.loads(line) for line in open(HERE / "replies.jsonl", encoding="utf-8")]
    table = collections.defaultdict(collections.Counter)
    for record in records:
        outcome = score(record)
        table[record["variant"]][outcome] += 1
        if outcome not in ("detected", "correct"):
            print("read by hand:", record["model"], record["variant"], record["repetition"], outcome)
    for variant in sorted(table):
        print(variant, dict(table[variant]))


if __name__ == "__main__":
    main()
