"""Score adjudicated E2 observations, not raw prose or model judgments.

Usage: python score.py <observations.json> | --self-test
Observations stay outside this public repository. See README for the input
contract, independent adjudication, and the draft's limits. No E2 data exists yet.
"""
import argparse
import collections
import csv
import json
import math
import pathlib
import random
import unittest

HERE = pathlib.Path(__file__).resolve().parent
ARMS = ("code", "specification", "review")


def observable(arm, category):
    if arm == "code":
        return category in ("clean", "body", "wrong_complete")
    if arm == "specification":
        return category != "body"
    return True


def validate(row, key):
    if row["arm"] not in ARMS or row["task"] not in key:
        raise ValueError("unknown arm or task")
    for name, low, high in [("seconds", 0, 480), ("confidence", -1, 100)]:
        value = row[name]
        if type(value) not in (int, float) or not math.isfinite(value) or not low < value <= high:
            raise ValueError(f"invalid {name}")
    if row["disposition"] not in ("accept", "request_change", "insufficient_information", "not_observable"):
        raise ValueError("invalid disposition")
    if not isinstance(row["reviewer"], str) or not row["reviewer"]:
        raise ValueError("missing reviewer")
    if type(row["finished"]) is not bool:
        raise ValueError("missing completion record")
    first = row["first_valid_seconds"]
    if first is not None and (type(first) not in (int, float) or not math.isfinite(first) or not 0 <= first <= row["seconds"]):
        raise ValueError("invalid first-find time")
    for claim in row["claims"]:
        if type(claim["valid"]) is not bool or type(claim["witness_valid"]) is not bool:
            raise ValueError("claims must be independently adjudicated")
        if not isinstance(claim["defect"], str):
            raise ValueError("claim needs an adjudicated defect identifier")


def summarize(rows, key):
    found, false, wrong, seconds = 0, 0, 0, 0
    recalls = []
    clean, clean_rejected, false_accepted = 0, 0, 0
    first_times = []
    for row in rows:
        validate(row, key)
        target = key[row["task"]]
        valid = set()
        for claim in row["claims"]:
            if target["defect"] is not None and claim["defect"] == target["defect"] and claim["valid"] and claim["witness_valid"]:
                valid.add(claim["defect"])
            else:
                false += 1
        found += len(valid)
        seconds += row["seconds"]
        if valid and row["first_valid_seconds"] is not None:
            first_times.append(row["first_valid_seconds"])
        if target["defect"] is None:
            clean += 1
            clean_rejected += int(row["disposition"] == "request_change")
        if observable(row["arm"], target["category"]) and target["defect"] is not None:
            recalls.append(bool(valid))
            false_accepted += int(row["disposition"] == "accept")
            wrong += int(row["disposition"] == "accept" and row["confidence"] >= 80)
    return dict(tasks=len(rows), true_finds=found, false_positives=false,
                active_minutes=seconds / 60, true_per_minute=found * 60 / seconds if seconds else None,
                false_per_task=false / len(rows) if rows else None,
                observable_recall=sum(recalls) / len(recalls) if recalls else None,
                high_confidence_wrong_acceptances=wrong,
                high_confidence_wrong_acceptance_rate=wrong / len(recalls) if recalls else None,
                precision=found / (found + false) if found + false else None,
                clean_control_rejection_rate=clean_rejected / clean if clean else None,
                false_acceptance_rate=false_accepted / len(recalls) if recalls else None,
                first_valid_seconds=first_times,
                unfinished_tasks=sum(not r["finished"] for r in rows),
                completion_seconds=[r["seconds"] for r in rows if r["finished"]])


def disposition_correct(row, target):
    if not observable(row["arm"], target["category"]) or row["disposition"] in ("insufficient_information", "not_observable"):
        return None
    if row["disposition"] == "accept":
        return target["defect"] is None
    return any(c["valid"] and c["witness_valid"] and c["defect"] == target["defect"] for c in row["claims"])


def interval(values):
    values = sorted(values)
    return [values[int((len(values) - 1) * q)] for q in (0.0125, 0.9875)]


def analyze(rows, key, repetitions=10000):
    seen = set()
    grouped = {arm: [] for arm in ARMS}
    for row in sorted(rows, key=lambda r: (r["reviewer"], r["task"])):
        validate(row, key)
        identity = (row["reviewer"], row["task"])
        if identity in seen:
            raise ValueError("duplicate reviewer/task")
        seen.add(identity)
        grouped[row["arm"]].append(row)
    summaries = {a: summarize(grouped[a], key) for a in ARMS}
    contrasts = {}
    rng = random.Random(4607)
    strata = {}
    for arm in ARMS:
        buckets = collections.defaultdict(lambda: collections.defaultdict(list))
        for row in grouped[arm]:
            buckets[row["stratum"]][row["reviewer"]].append(row)
        strata[arm] = [list(reviewers.values()) for reviewers in buckets.values()]
    rate_diffs = {a: [] for a in ARMS[:2]}
    false_diffs = {a: [] for a in ARMS[:2]}
    if all(grouped.values()):
        # Resample whole reviewers within arm and experience stratum, retaining
        # both tasks together. Never treat the two tasks as independent people.
        for _ in range(repetitions):
            sample = {}
            for arm in ARMS:
                draw = [row for bucket in strata[arm] for _ in bucket for row in rng.choice(bucket)]
                sample[arm] = summarize(draw, key)
            for baseline in ARMS[:2]:
                rate_diffs[baseline].append(sample["review"]["true_per_minute"] - sample[baseline]["true_per_minute"])
                false_diffs[baseline].append(sample["review"]["false_per_task"] - sample[baseline]["false_per_task"])
    for baseline in ARMS[:2]:
        b, r = summaries[baseline], summaries["review"]
        contrasts[baseline] = dict(
            rate_ratio=r["true_per_minute"] / b["true_per_minute"] if b["true_per_minute"] and r["true_per_minute"] is not None else None,
            rate_difference_interval=interval(rate_diffs[baseline]) if rate_diffs[baseline] else None,
            false_positive_difference_interval=interval(false_diffs[baseline]) if false_diffs[baseline] else None)
    domain = {d: {arm: summarize([r for r in grouped[arm] if key[r["task"]]["domain"] == d], key) for arm in ARMS} for d in ("gate", "cedar")}
    common = {arm: summarize([r for r in grouped[arm] if key[r["task"]]["category"] == "wrong_complete"], key) for arm in ARMS}
    complete = len(rows) == 180 and len({r["reviewer"] for r in rows}) == 90 and all(summaries[a]["tasks"] == 60 for a in ARMS)
    evaluable = complete and all(c["rate_ratio"] is not None for c in contrasts.values()) and all(domain[d][a]["true_per_minute"] is not None for d in domain for a in ARMS) and all(common[a]["observable_recall"] is not None for a in ARMS)
    passed = None
    if evaluable:
        passed = all(c["rate_ratio"] >= 1.25 and c["rate_difference_interval"][0] > 0 and c["false_positive_difference_interval"][1] <= 0.10 for c in contrasts.values())
        passed = passed and all(domain[d]["review"]["true_per_minute"] > domain[d][a]["true_per_minute"] for d in domain for a in ARMS[:2])
        passed = passed and all(common["review"]["observable_recall"] >= common[a]["observable_recall"] for a in ARMS[:2])
        passed = passed and all(summaries["review"]["high_confidence_wrong_acceptance_rate"] <= summaries[a]["high_confidence_wrong_acceptance_rate"] for a in ARMS[:2])
    confidence = {arm: {label: [r["confidence"] for r in grouped[arm] if disposition_correct(r, key[r["task"]]) is correct] for label, correct in [("correct", True), ("incorrect", False), ("unscored", None)]} for arm in ARMS}
    categories = {c: {a: summarize([r for r in grouped[a] if key[r["task"]]["category"] == c], key) for a in ARMS} for c in sorted({v["category"] for v in key.values()})}
    return dict(status="analysis_only_protocol_not_registered", arms=summaries, domains=domain, categories=categories, common_observability=common,
                contrasts=contrasts, confidence_by_correct_disposition=confidence,
                bootstrap=dict(seed=4607, repetitions=repetitions, nominal_simultaneous_coverage=0.95),
                complete=complete, pass_mark_met=passed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("observations", nargs="?", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.discover(str(HERE), pattern="test_*.py")
        if not unittest.TextTestRunner().run(suite).wasSuccessful():
            raise SystemExit(1)
        return
    if args.observations is None:
        parser.error("provide observations or --self-test")
    key = {t["task"]: t for t in json.loads((HERE / "answer-key.json").read_text(encoding="utf-8"))}
    with (HERE / "allocation.csv").open(encoding="utf-8", newline="") as stream:
        allocation = {r["reviewer"]: r for r in csv.DictReader(stream)}
    rows = json.loads(args.observations.read_text(encoding="utf-8"))
    for row in rows:
        slot = allocation[row["reviewer"]]
        if row["arm"] != slot["arm"] or row["stratum"] != slot["stratum"] or row["task"] not in (slot["first"], slot["second"]):
            raise ValueError("observation does not match frozen allocation")
    print(json.dumps(analyze(rows, key), indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
