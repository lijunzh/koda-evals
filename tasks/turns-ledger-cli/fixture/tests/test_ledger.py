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


class LedgerTest(unittest.TestCase):
    def test_add_then_list(self):
        with tempfile.TemporaryDirectory() as d:
            ledger = os.path.join(d, "l.json")
            run("add", "2026-11-02", "12.50", "groceries", "market", ledger=ledger)
            out = run("list", ledger=ledger)
            self.assertEqual(out, "2026-11-02       12.50  groceries  market\n")


if __name__ == "__main__":
    unittest.main()
