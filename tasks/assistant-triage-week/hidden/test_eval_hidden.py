"""Hidden check for assistant-triage-week: todo.md lists exactly the six
things Sam has to do this week, each with its deadline. The traps (a
registration due on the 30th, an autopay bill, a book-club request
withdrawn two days later, a phishing mail, snacks asked of Chris) must not
be on it."""

import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = [
    ("2026-11-12", r"jordan|dinner"),
    ("2026-11-13", r"permission|field trip"),
    ("2026-11-13", r"dentist|bright smile|appointment"),
    ("2026-11-14", r"library|wild robot"),
    ("2026-11-14", r"prescription|pharmacy|carerx"),
    ("2026-11-15", r"lease|unit 4b|harbor"),
]
TRAPS = r"registration|dmv|autopay|portland power|power bill|book club|host|payfast|verify your|snack|orange"

def inputs_unchanged(test):
    """mail/ and calendar/ are Sam's data: the task only reads them."""
    try:
        out = subprocess.run(["git", "status", "--porcelain", "--", "mail", "calendar"],
                             cwd=ROOT, capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return
    if out.returncode == 0:
        test.assertEqual(out.stdout.strip(), "", "mail/ or calendar/ was changed")


class EvalHiddenTest(unittest.TestCase):
    def items(self):
        path = ROOT / "todo.md"
        self.assertTrue(path.exists(), "no todo.md")
        return [l.strip() for l in path.read_text(encoding="utf-8").splitlines()
                if re.match(r"\s*[-*]\s", l)]

    def test_eval_hidden_each_thing_to_do_with_its_date(self):
        items = self.items()
        for date, pattern in EXPECTED:
            hits = [l for l in items if re.search(pattern, l, re.I)]
            self.assertEqual(len(hits), 1, f"want one line for /{pattern}/, got {hits}")
            self.assertIn(date, hits[0], f"wrong date: {hits[0]}")

    def test_eval_hidden_nothing_else(self):
        items = self.items()
        traps = [l for l in items if re.search(TRAPS, l, re.I)]
        self.assertEqual(traps, [], "not Sam's to do this week")
        self.assertEqual(len(items), len(EXPECTED), f"want {len(EXPECTED)} lines: {items}")

    def test_eval_hidden_inputs_unchanged(self):
        inputs_unchanged(self)


if __name__ == "__main__":
    unittest.main()
