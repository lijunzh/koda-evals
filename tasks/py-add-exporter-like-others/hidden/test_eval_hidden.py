"""Hidden tests: TSV is added the way CSV and JSON are, everywhere the
project lists its formats, and follows CONTRIBUTING.md's rules for text
formats. Nothing checks names the prompt and the project leave open (a
module's name, a description's wording, where in a table a row goes)."""

import contextlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from decimal import Decimal

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def read(rel):
    with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
        return f.read()


class EvalHiddenExporter(unittest.TestCase):
    def test_registered_like_the_others(self):
        from reportkit.exporters import EXPORTERS
        from reportkit.exporters.base import Exporter

        cls = EXPORTERS["tsv"]
        self.assertTrue(issubclass(cls, Exporter))
        self.assertEqual(cls.name, "tsv")
        self.assertEqual(cls.extension, ".tsv")
        self.assertEqual(cls.media_type, "text/tab-separated-values")

    def test_follows_the_text_format_rules(self):
        from reportkit.exporters import get_exporter

        records = [
            {"day": date(2026, 11, 2), "clinic": "North", "visits": 41, "fee": Decimal("12.50"), "note": None},
            {"day": date(2026, 11, 3), "clinic": "South, annex", "visits": 12, "fee": Decimal("0"),
             "note": "tab\there, line\nbreak, cr\rhere, back\\slash, literal \\t"},
        ]
        out = get_exporter("tsv").export(records, ["day", "clinic", "visits", "fee", "note"])
        self.assertTrue(out.endswith("\n"), repr(out))
        lines = out[:-1].split("\n")
        self.assertEqual(len(lines), 3, repr(out))
        self.assertEqual(lines[0].split("\t"), ["day", "clinic", "visits", "fee", "note"])
        self.assertEqual(lines[1].split("\t"), ["2026-11-02", "North", "41", "12.50", ""])
        self.assertEqual(
            lines[2].split("\t"),
            ["2026-11-03", "South, annex", "12", "0",
             "tab\\there, line\\nbreak, cr\\rhere, back\\\\slash, literal \\\\t"],
        )

    def test_column_order_is_kept(self):
        from reportkit.exporters import get_exporter

        out = get_exporter("tsv").export([{"a": 1, "b": 2}], ["b", "a"])
        self.assertEqual(out, "b\ta\n2\t1\n")


class EvalHiddenCli(unittest.TestCase):
    def test_the_cli_exports_tsv(self):
        from reportkit.cli import main

        with tempfile.TemporaryDirectory() as d:
            src, dst = os.path.join(d, "in.json"), os.path.join(d, "out.tsv")
            with open(src, "w", encoding="utf-8") as f:
                json.dump([{"day": "2026-11-03", "note": "a\tb"}], f)
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(["--format", "tsv", src, dst]), 0)
            with open(dst, encoding="utf-8", newline="") as f:
                self.assertEqual(f.read(), "day\tnote\n2026-11-03\ta\\tb\n")

    def test_list_formats_names_tsv(self):
        from reportkit.cli import main

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(main(["--list-formats"]), 0)
        lines = [l for l in out.getvalue().splitlines() if l.split()[:1] == ["tsv"]]
        self.assertEqual(len(lines), 1, out.getvalue())
        self.assertGreater(len(lines[0].split()), 1, "tsv needs a description, as csv and json have")


class EvalHiddenProject(unittest.TestCase):
    def test_documented_in_the_formats_table(self):
        rows = [l for l in read("docs/formats.md").splitlines()
                if l.startswith("|") and re.search(r"\btsv\b", l, re.I)]
        self.assertEqual(len(rows), 1, "one table row for TSV")
        self.assertIn(".tsv", rows[0])
        self.assertIn("text/tab-separated-values", rows[0])

    def test_in_the_changelog_under_unreleased_added(self):
        text = read("CHANGELOG.md")
        m = re.search(r"^## Unreleased\n(.*?)(?=^## )", text, re.M | re.S)
        self.assertIsNotNone(m, "an Unreleased section")
        added = re.search(r"^### Added\n(.*?)(?=^### |\Z)", m.group(1), re.M | re.S)
        self.assertIsNotNone(added, "an Added section under Unreleased")
        self.assertRegex(added.group(1), r"(?i)\btsv\b|tab-separated")
        self.assertIn("## 0.3.0", text, "earlier releases are kept")

    def test_has_its_own_passing_test_module(self):
        self.assertTrue(os.path.isfile(os.path.join(ROOT, "tests", "test_tsv.py")))
        run = subprocess.run([sys.executable, "-m", "unittest", "-v", "tests.test_tsv"],
                             cwd=ROOT, capture_output=True, text=True, timeout=60)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertRegex(run.stderr, r"Ran [1-9]\d* tests?")

    def test_the_existing_tests_still_pass(self):
        run = subprocess.run([sys.executable, "-m", "unittest", "tests.test_csv", "tests.test_json",
                              "tests.test_cli"], cwd=ROOT, capture_output=True, text=True, timeout=60)
        self.assertEqual(run.returncode, 0, run.stderr)


if __name__ == "__main__":
    unittest.main()
