"""Turn replies into a scoring sheet, then apply the registered pass mark.

Usage:
  python score.py extract     writes answers.csv, one row per answer, with an
                              empty `correct` column to fill in by hand
  python score.py summarize   reads the filled answers.csv and reports each
                              question's rate and the verdict

Scoring is done by hand against the answer key in README.md, because the
answers are free text. Every judgment stays in answers.csv beside the quoted
answer, so anyone can audit it.
"""

import collections
import csv
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).parent
DIAGNOSTIC = {"S3.2"}
INTENT = "S1.4"


def extract() -> None:
    records = [json.loads(line) for line in open(HERE / "replies.jsonl", encoding="utf-8")]
    with open(HERE / "answers.csv", "w", encoding="utf-8", newline="") as out:
        sheet = csv.writer(out)
        sheet.writerow(["model", "repetition", "question", "answer", "correct"])
        for record in records:
            text = re.sub(r"[*`#]", "", record["reply"])
            parts = re.split(r"(?m)^\s*(S\d\.\d)\s*[:.)-]", text)
            answers = {parts[i]: " ".join(parts[i + 1].split()) for i in range(1, len(parts) - 1, 2)}
            labels = re.findall(r"(?m)^(S\d\.\d) ", (HERE / "snippets" / f"{record['snippet']}.txt")
                                .read_text(encoding="utf-8").split("\n---\n")[1])
            for label in labels:
                sheet.writerow([record["model"], record["repetition"], label, answers.get(label, ""), ""])


def summarize() -> None:
    rows = list(csv.DictReader(open(HERE / "answers.csv", encoding="utf-8")))
    unscored = [r for r in rows if r["correct"] not in ("yes", "no")]
    if unscored:
        sys.exit(f"{len(unscored)} answers have no judgment in the `correct` column")
    by_question = collections.defaultdict(list)
    for row in rows:
        by_question[row["question"]].append(row["correct"] == "yes")
    for question, marks in sorted(by_question.items()):
        tag = " (diagnostic)" if question in DIAGNOSTIC else ""
        print(f"{question}: {sum(marks)}/{len(marks)} = {sum(marks) / len(marks):.0%}{tag}")
    scored = [m for q, marks in by_question.items() if q not in DIAGNOSTIC for m in marks]
    overall = sum(scored) / len(scored)
    weakest = min(sum(m) / len(m) for q, m in by_question.items() if q not in DIAGNOSTIC)
    intent = sum(by_question[INTENT]) / len(by_question[INTENT])
    passed = overall >= 0.90 and weakest >= 0.70 and intent >= 0.90
    print(f"overall {overall:.0%} (needs 90%), weakest question {weakest:.0%} (needs 70%), "
          f"{INTENT} {intent:.0%} (needs 90%): {'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    {"extract": extract, "summarize": summarize}[sys.argv[1]]()
