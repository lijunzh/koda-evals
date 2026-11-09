import json
import unittest
from datetime import date
from decimal import Decimal

from reportkit.exporters import EXPORTERS, get_exporter


class JsonTest(unittest.TestCase):
    def test_registered(self):
        self.assertEqual(EXPORTERS["json"].media_type, "application/json")

    def test_values(self):
        out = get_exporter("json").export(
            [{"day": date(2026, 11, 3), "amount": Decimal("1.50"), "n": None}], ["day", "amount", "n"])
        self.assertEqual(json.loads(out), [{"day": "2026-11-03", "amount": "1.50", "n": None}])
