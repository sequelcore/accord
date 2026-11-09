"""Synthetic scoring oracles; no participant observations."""
import unittest
import copy
import csv
import json
import pathlib
import score


class ScoringTests(unittest.TestCase):
    def setUp(self):
        self.key = {"R01": {"category": "clean", "domain": "gate", "defect": None}, "R02": {"category": "body", "domain": "gate", "defect": "gate_body"}}
        self.row = dict(reviewer="P001", arm="review", stratum="experienced", task="R02", seconds=30, disposition="request_change", confidence=90,
                        finished=True, first_valid_seconds=20, claims=[dict(defect="gate_body", valid=True, witness_valid=True)])

    def test_unique_true_finds_are_per_active_minute(self):
        self.row["claims"] *= 2
        summary = score.summarize([self.row], self.key)
        self.assertEqual(summary["true_finds"], 1)
        self.assertEqual(summary["false_positives"], 0)
        self.assertEqual(summary["true_per_minute"], 2)
        self.assertEqual(summary["precision"], 1)
        self.assertEqual(summary["first_valid_seconds"], [20])

    def test_invalid_and_unwitnessed_claims_are_false_positives(self):
        self.row["claims"] = [dict(defect="gate_body", valid=True, witness_valid=False), dict(defect="invented", valid=True, witness_valid=True)]
        summary = score.summarize([self.row], self.key)
        self.assertEqual(summary["true_finds"], 0)
        self.assertEqual(summary["false_positives"], 2)

    def test_clean_controls_have_no_true_defect(self):
        self.row["task"] = "R01"
        self.row["disposition"] = "accept"
        summary = score.summarize([self.row], self.key)
        self.assertEqual(summary["true_finds"], 0)
        self.assertEqual(summary["false_positives"], 1)
        self.assertEqual(summary["high_confidence_wrong_acceptances"], 0)

    def test_unobservable_body_error_is_not_wrong_acceptance_in_spec_arm(self):
        self.row.update(arm="specification", disposition="accept", claims=[])
        self.assertEqual(score.summarize([self.row], self.key)["high_confidence_wrong_acceptances"], 0)
        self.row["arm"] = "review"
        self.assertEqual(score.summarize([self.row], self.key)["high_confidence_wrong_acceptances"], 1)

    def test_confidence_for_unobservable_dispositions_is_unscored(self):
        self.row.update(arm="specification", disposition="accept", claims=[])
        result = score.analyze([self.row], self.key, repetitions=10)
        self.assertEqual(result["confidence_by_correct_disposition"]["specification"]["unscored"], [90])

    def test_invalid_measurements_are_rejected(self):
        for field, value in [("seconds", 0), ("seconds", 481), ("confidence", 101), ("confidence", True)]:
            bad = dict(self.row, **{field: value})
            with self.assertRaises(ValueError):
                score.summarize([bad], self.key)

    def test_bootstrap_repeats_by_seed_and_zero_baseline_is_not_infinite_gain(self):
        rows = [dict(self.row, reviewer="P" + str(i), arm=arm, claims=[] if arm != "review" else self.row["claims"]) for i, arm in enumerate(score.ARMS)]
        first = score.analyze(rows, self.key, repetitions=100)
        self.assertEqual(first, score.analyze(rows, self.key, repetitions=100))
        self.assertIsNone(first["contrasts"]["code"]["rate_ratio"])
        self.assertIsNone(first["pass_mark_met"])

    def test_complete_positive_control_and_false_positive_guardrail(self):
        # Entirely synthetic observations exercise the draft decision rule.
        here = pathlib.Path(__file__).parent
        key = {t["task"]: t for t in json.loads((here / "answer-key.json").read_text(encoding="utf-8"))}
        with (here / "allocation.csv").open(encoding="utf-8", newline="") as stream:
            slots = list(csv.DictReader(stream))
        rows = []
        baseline_found = set()
        for slot in slots:
            for task in (slot["first"], slot["second"]):
                target = key[task]
                found = target["defect"] is not None and (slot["arm"] == "review" or slot["arm"] not in baseline_found and score.observable(slot["arm"], target["category"]))
                if found and slot["arm"] != "review":
                    baseline_found.add(slot["arm"])
                rows.append(dict(reviewer=slot["reviewer"], arm=slot["arm"], stratum=slot["stratum"], task=task,
                                 seconds=60, finished=True, first_valid_seconds=45 if found else None, confidence=80 if found else 50,
                                 disposition="request_change" if found else "insufficient_information",
                                 claims=[dict(defect=target["defect"], valid=True, witness_valid=True)] if found else []))
        result = score.analyze(rows, key, repetitions=100)
        self.assertTrue(result["complete"])
        self.assertTrue(result["pass_mark_met"])
        noisy = copy.deepcopy(rows)
        for row in noisy:
            if row["arm"] == "review":
                row["claims"].append(dict(defect="invalid", valid=False, witness_valid=False))
        self.assertFalse(score.analyze(noisy, key, repetitions=100)["pass_mark_met"])
        self.assertIsNone(score.analyze(rows[:-1], key, repetitions=100)["pass_mark_met"])


if __name__ == "__main__":
    unittest.main()
