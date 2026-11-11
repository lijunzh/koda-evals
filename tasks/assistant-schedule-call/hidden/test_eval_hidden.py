"""Hidden check for assistant-schedule-call: slots.txt holds exactly the
45-minute starts next week that suit Priya (London, 13:00-18:00, not
Friday) and Sam (09:00-17:00 Pacific on the hour or half hour, Wednesday
mornings free, the calendar). London is 8 hours ahead in late November,
so only 09:00-10:00 Pacific overlaps: Monday and Tuesday at 09:00.
Thursday 09:00 is taken by a webinar stored in UTC (17:00Z)."""

import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = {"2026-11-16 09:00", "2026-11-17 09:00"}

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
    def test_eval_hidden_exactly_the_times_that_work(self):
        path = ROOT / "slots.txt"
        self.assertTrue(path.exists(), "no slots.txt")
        found = set(re.findall(r"\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}", path.read_text(encoding="utf-8")))
        found = {f.replace("T", " ") for f in found}
        self.assertEqual(found, EXPECTED)

    def test_eval_hidden_inputs_unchanged(self):
        inputs_unchanged(self)


if __name__ == "__main__":
    unittest.main()
