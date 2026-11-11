"""Hidden tests: any payee or memo text round-trips, the count is of
transactions, and spreadsheets read the export. Nothing here depends on the
exact quoting style or line ending, only on what a CSV reader gets back.

No field holds a lone "\r": Python 3.9's csv writer leaves one unquoted
when the line terminator is "\n", and 3.12+ quotes it, so the same fix would
pass on one Python and fail on the other."""

import csv
import os
import tempfile
import unittest
from datetime import date
from decimal import Decimal

from ledger.export import write_report
from ledger.importer import read_report
from ledger.model import Transaction
from ledger.summary import count_rows

TRICKY = [
    'Dinner with "Sam", split 3 ways\npaid back Friday',
    "two\r\nlines, windows style",
    '"',
    '""',
    'ends with a quote"',
    "  spaces at both ends  ",
    "tab\there",
    "a,b,c",
    "",
    "line separator and next\u0085line",
    "café ☕ 日本",
    "trailing newline\n",
]


def txn(i, payee, memo):
    return Transaction(date(2026, 10, 1 + i % 28), payee, memo, Decimal(f"{i + 1}.{i % 100:02d}"))


class EvalHiddenRoundTrip(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "r.csv")

    def tearDown(self):
        self.dir.cleanup()

    def test_every_memo_round_trips(self):
        txns = [txn(i, "Payee", memo) for i, memo in enumerate(TRICKY)]
        write_report(self.path, txns)
        self.assertEqual(read_report(self.path), txns)

    def test_every_payee_round_trips(self):
        txns = [txn(i, payee, "memo") for i, payee in enumerate(TRICKY)]
        write_report(self.path, txns)
        self.assertEqual(read_report(self.path), txns)

    def test_the_count_is_of_transactions(self):
        txns = [txn(i, p, m) for i, (p, m) in enumerate(zip(TRICKY, reversed(TRICKY)))]
        write_report(self.path, txns)
        self.assertEqual(count_rows(self.path), len(txns))

    def test_a_spreadsheet_reads_the_same_fields(self):
        txns = [txn(i, "P, Inc.", memo) for i, memo in enumerate(TRICKY)]
        write_report(self.path, txns)
        # A byte-order mark is allowed: Excel needs one to read UTF-8.
        with open(self.path, encoding="utf-8-sig", newline="") as f:
            rows = list(csv.reader(f))
        self.assertEqual(rows[0], ["date", "payee", "memo", "amount"])
        self.assertEqual([r[2] for r in rows[1:]], TRICKY)
        self.assertEqual([r[1] for r in rows[1:]], ["P, Inc."] * len(TRICKY))

    def test_reports_written_before_the_fix_still_load(self):
        with open(self.path, "w", encoding="utf-8", newline="") as f:
            f.write('date,payee,memo,amount\n2026-10-02,Corner Market,weekly shop,84.20\n'
                    '2026-10-09,"Power Co, Inc.",,118.45\n')
        self.assertEqual(read_report(self.path), [
            Transaction(date(2026, 10, 2), "Corner Market", "weekly shop", Decimal("84.20")),
            Transaction(date(2026, 10, 9), "Power Co, Inc.", "", Decimal("118.45")),
        ])
        self.assertEqual(count_rows(self.path), 2)

    def test_the_reported_example(self):
        memo = 'Dinner with "Sam", split 3 ways\npaid back Friday'
        txns = [txn(0, "Corner Market", "weekly shop"), txn(1, "Noodle Bar", memo),
                txn(2, "Power Co, Inc.", "")]
        write_report(self.path, txns)
        self.assertEqual(read_report(self.path), txns)
        self.assertEqual(count_rows(self.path), 3)


if __name__ == "__main__":
    unittest.main()
