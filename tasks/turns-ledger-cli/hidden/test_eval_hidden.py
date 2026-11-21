"""Hidden tests for turns-ledger-cli: the state after all four turns.

Turn 1 asked for `summary --month`, sorted by total; turn 4 changed that to
alphabetical with a TOTAL line, so the sort checked here is turn 4's. Turn 2
(whole cents, old files migrated) and turn 3 (--category) must still hold.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def run(*args, ledger):
    env = dict(os.environ, LEDGER=ledger)
    return subprocess.run([sys.executable, os.path.join(HERE, "ledger.py"), *args],
                          env=env, capture_output=True, text=True, check=True).stdout


class Ledger:
    def __init__(self, test):
        self.dir = tempfile.TemporaryDirectory()
        test.addCleanup(self.dir.cleanup)
        self.path = os.path.join(self.dir.name, "ledger.json")

    def add(self, *args):
        run("add", *args, ledger=self.path)

    def run(self, *args):
        return run(*args, ledger=self.path)


class TurnsLedgerHidden(unittest.TestCase):
    def sample(self):
        ledger = Ledger(self)
        ledger.add("2026-11-02", "12.50", "groceries", "market")
        ledger.add("2026-11-03", "40", "transport")
        ledger.add("2026-11-09", "7.25", "groceries")
        ledger.add("2026-11-12", "3.10", "coffee")
        ledger.add("2026-10-30", "99.99", "groceries", "last month")
        return ledger

    def test_summary_is_alphabetical_with_a_total(self):
        out = self.sample().run("summary", "--month", "2026-11")
        self.assertEqual(out, "coffee 3.10\ngroceries 19.75\ntransport 40.00\nTOTAL 62.85\n")

    def test_category_filter_keeps_the_total_line(self):
        out = self.sample().run("summary", "--month", "2026-11", "--category", "groceries")
        self.assertEqual(out, "groceries 19.75\nTOTAL 19.75\n")

    def test_amounts_are_stored_as_whole_cents(self):
        ledger = Ledger(self)
        ledger.add("2026-11-02", "0.10", "coffee")
        ledger.add("2026-11-03", "0.20", "coffee")
        with open(ledger.path) as f:
            entries = json.load(f)
        self.assertEqual([e.get("cents") for e in entries], [10, 20])
        self.assertTrue(all("amount" not in e for e in entries))
        self.assertTrue(all(type(e["cents"]) is int for e in entries))
        self.assertEqual(ledger.run("summary", "--month", "2026-11"),
                         "coffee 0.30\nTOTAL 0.30\n")

    def test_an_old_ledger_still_loads_and_is_migrated(self):
        ledger = Ledger(self)
        with open(ledger.path, "w") as f:
            json.dump([
                {"date": "2026-11-01", "amount": 0.1, "category": "coffee", "note": ""},
                {"date": "2026-11-02", "amount": 0.2, "category": "coffee", "note": ""},
                {"date": "2026-11-05", "amount": 19.99, "category": "books", "note": "novel"},
            ], f)
        self.assertEqual(ledger.run("summary", "--month", "2026-11"),
                         "books 19.99\ncoffee 0.30\nTOTAL 20.29\n")
        ledger.add("2026-11-06", "1.01", "coffee")
        with open(ledger.path) as f:
            entries = json.load(f)
        self.assertEqual([e.get("cents") for e in entries], [10, 20, 1999, 101])
        self.assertTrue(all("amount" not in e for e in entries))

    def test_list_reads_the_same_as_before(self):
        out = self.sample().run("list")
        self.assertEqual(out.splitlines()[0], "2026-11-02       12.50  groceries  market")
        self.assertEqual(out.splitlines()[1], "2026-11-03       40.00  transport")


if __name__ == "__main__":
    unittest.main()
