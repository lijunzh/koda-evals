import os
import tempfile
import unittest
from datetime import date
from decimal import Decimal

from ledger.export import write_report
from ledger.importer import read_report
from ledger.model import Transaction
from ledger.summary import count_rows


class LedgerTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "r.csv")

    def tearDown(self):
        self.dir.cleanup()

    def test_round_trip(self):
        txns = [
            Transaction(date(2026, 10, 2), "Corner Market", "weekly shop", Decimal("84.20")),
            Transaction(date(2026, 10, 9), "Power Co, Inc.", "", Decimal("118.45")),
        ]
        self.assertEqual(write_report(self.path, txns), 2)
        self.assertEqual(read_report(self.path), txns)
        self.assertEqual(count_rows(self.path), 2)

    def test_empty_report(self):
        write_report(self.path, [])
        self.assertEqual(read_report(self.path), [])
        self.assertEqual(count_rows(self.path), 0)


if __name__ == "__main__":
    unittest.main()
