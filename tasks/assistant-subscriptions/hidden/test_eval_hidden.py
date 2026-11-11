"""Hidden check for assistant-subscriptions: subscriptions.csv lists the
six subscriptions still running, each with its next charge. StreamFlix's
next charge is the raised price; NewsDaily's trial turns paid on the 13th;
Tunewave's latest receipt wins; FitTrack is cancelled and PhotoPro was a
one-time purchase."""

import csv
import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = {
    "streamflix": (17.99, "monthly", "2026-11-21"),
    "tunewave": (10.99, "monthly", "2026-12-03"),
    "cloudbox": (99.99, "yearly", "2027-03-02"),
    "newsdaily": (4.99, "monthly", "2026-11-13"),
    "peak": (65.00, "monthly", "2026-12-01"),
    "samrivera": (14.00, "yearly", "2027-08-19"),
}
ALIASES = {"namenest": "samrivera"}

def inputs_unchanged(test):
    """mail/ and calendar/ are Sam's data: the task only reads them."""
    try:
        out = subprocess.run(["git", "status", "--porcelain", "--", "mail", "calendar"],
                             cwd=ROOT, capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return
    if out.returncode == 0:
        test.assertEqual(out.stdout.strip(), "", "mail/ or calendar/ was changed")


def period(text):
    t = text.strip().lower()
    if t.startswith("month"):
        return "monthly"
    if t.startswith(("year", "annual")):
        return "yearly"
    return t


class EvalHiddenTest(unittest.TestCase):
    def rows(self):
        path = ROOT / "subscriptions.csv"
        self.assertTrue(path.exists(), "no subscriptions.csv")
        with path.open(newline="", encoding="utf-8-sig") as f:
            reader = csv.DictReader(f)
            self.assertEqual([h.strip().lower() for h in reader.fieldnames or []],
                             ["service", "amount", "period", "next_renewal"])
            return [{k.strip().lower(): (v or "").strip() for k, v in r.items()} for r in reader]

    def test_eval_hidden_each_subscription(self):
        rows = self.rows()
        for key, (amount, per, date) in EXPECTED.items():
            hits = [r for r in rows if key in r["service"].lower()
                    or any(a in r["service"].lower() for a, k in ALIASES.items() if k == key)]
            self.assertEqual(len(hits), 1, f"want one row for {key}: {hits}")
            r = hits[0]
            self.assertAlmostEqual(float(re.sub(r"[^0-9.]", "", r["amount"])), amount, places=2, msg=str(r))
            self.assertEqual(period(r["period"]), per, str(r))
            self.assertEqual(r["next_renewal"], date, str(r))

    def test_eval_hidden_nothing_else(self):
        rows = self.rows()
        self.assertEqual([r for r in rows if re.search(r"fittrack|photopro", r["service"], re.I)], [])
        self.assertEqual(len(rows), len(EXPECTED), str(rows))

    def test_eval_hidden_inputs_unchanged(self):
        inputs_unchanged(self)


if __name__ == "__main__":
    unittest.main()
