"""A machine consumer stops on unknown schemas, codes, or unusable witnesses."""
import copy
import unittest
import materials


class WireTests(unittest.TestCase):
    def setUp(self):
        self.report = dict(receipt=4, status="checked", definitions=[], diagnostics=[dict(code="ACC0012", evidence="/definitions")])

    def test_known_wire_contract_has_resolvable_evidence(self):
        materials.validate_wire(self.report)

    def test_unknown_schema_and_code_are_rejected(self):
        for update in (dict(receipt=2), dict(diagnostics=[dict(code="ACC9999", evidence="/definitions")])):
            with self.assertRaises(AssertionError):
                materials.validate_wire(dict(self.report, **update))

    def test_an_unusable_witness_is_not_accepted(self):
        report = copy.deepcopy(self.report)
        report["definitions"] = [dict(witness=dict(source="bad", value=None, valid=False))]
        with self.assertRaises(AssertionError):
            materials.validate_wire(report)


if __name__ == "__main__":
    unittest.main()
