"""Score the replies of version 2 and apply its registered rules.

Usage:
  python score.py extract     writes answers.csv, one row per answer. Questions
                              with a value or yes/no key are marked by rule;
                              the rest get an empty `correct` to mark by hand
  python score.py summarize   reads the filled answers.csv, reports each form's
                              rates, its pass or fail, and the form the
                              decision rule picks

The keys and the rules are in README.md.
"""

import collections
import csv
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).parent

# Questions whose first named value decides: the key, in any spelling.
VALUES = {"S1.1": "Flat", "S1.2": "PerDay", "S2.1": "0", "S4.1": "Clear", "S5.1": "Pass", "S5.2": "Fail"}
# Questions whose first yes or no decides.
YES_NO = {"S1.4": "no", "S1.5": "no", "S2.2": "yes", "S3.1": "no", "S3.2": "no"}
DIAGNOSTIC = {"S1.5", "S3.2", "S5.4"}
INTENT = "S1.4"
CHOICE = "S4.2"
TOKENS = r"PerDay|Waived|Flat|Clear|Red|Green|Yellow|Pass|Fail|Honors|-?\d+"


def answers_of(reply: str) -> dict:
    text = re.sub(r"[*`#]", "", reply)
    parts = re.split(r"(?m)^\s*(S\d\.\d)\s*[:.)-]", text)
    return {parts[i]: " ".join(parts[i + 1].split()) for i in range(1, len(parts) - 1, 2)}


def by_rule(question: str, answer: str) -> str:
    if question in VALUES:
        # Calls repeated in the answer carry their arguments; drop them first.
        found = set(re.findall(rf"(?<![\w-])({TOKENS})\b", re.sub(r"\([^)]*\)", "", answer)))
        if len(found) != 1:
            return ""  # several values named: marked by hand
        return "yes" if found == {VALUES[question]} else "no"
    if question in YES_NO:
        found = re.search(r"\b(yes|no)\b", answer, re.IGNORECASE)
        return "yes" if found and found.group(1).lower() == YES_NO[question] else "no"
    return ""


def extract() -> None:
    records = [json.loads(line) for line in open(HERE / "replies.jsonl", encoding="utf-8")]
    with open(HERE / "answers.csv", "w", encoding="utf-8", newline="") as out:
        sheet = csv.writer(out, lineterminator="\n")
        sheet.writerow(["form", "model", "repetition", "question", "answer", "correct"])
        for record in records:
            answers = answers_of(record["reply"])
            questions = (HERE / "forms" / record["form"] / f"{record['snippet']}.txt").read_text(
                encoding="utf-8").split("\n---\n")[1]
            for label in re.findall(r"(?m)^(S\d\.\d) ", questions):
                answer = answers.get(label, "")
                sheet.writerow([record["form"], record["model"], record["repetition"], label, answer,
                                by_rule(label, answer)])


def summarize() -> None:
    rows = list(csv.DictReader(open(HERE / "answers.csv", encoding="utf-8")))
    unmarked = [r for r in rows if r["correct"] not in ("yes", "no")]
    if unmarked:
        sys.exit(f"{len(unmarked)} answers have no judgment in the `correct` column")
    marks = collections.defaultdict(lambda: collections.defaultdict(list))
    for row in rows:
        marks[row["form"]][row["question"]].append(row["correct"] == "yes")
    passing = {}
    for form, questions in sorted(marks.items()):
        rate = {q: sum(m) / len(m) for q, m in questions.items()}
        scored = [m for q, ms in questions.items() if q not in DIAGNOSTIC for m in ms]
        overall = sum(scored) / len(scored)
        weakest = min(r for q, r in rate.items() if q not in DIAGNOSTIC)
        passed = overall >= 0.90 and weakest >= 0.70 and rate[INTENT] >= 0.90
        print(f"== {form}: overall {overall:.0%}, weakest {weakest:.0%}, {INTENT} {rate[INTENT]:.0%}, "
              f"{CHOICE} {rate[CHOICE]:.0%}: {'PASS' if passed else 'FAIL'}")
        for q in sorted(rate):
            tag = " (diagnostic)" if q in DIAGNOSTIC else ""
            print(f"   {q}: {sum(questions[q])}/{len(questions[q])} = {rate[q]:.0%}{tag}")
        if passed:
            passing[form] = sum(questions[CHOICE])
    if not passing:
        print("No form passes: none is chosen.")
        return
    best = max(passing.values())
    close = sorted(f for f, n in passing.items() if best - n <= 2)
    print("Chosen:", close[0] if len(close) == 1 else f"within two answers on {CHOICE}: {', '.join(close)}; the maintainers choose")


if __name__ == "__main__":
    {"extract": extract, "summarize": summarize}[sys.argv[1]]()
